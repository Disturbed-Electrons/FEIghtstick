//! Nonexclusive HID transport with one report reader and one pending LED request per panel.
use std::{
    ffi::{CStr, CString},
    time::{Duration, Instant},
};

use hidapi::{HidApi, HidDevice, HidError};
use thiserror::Error;

use crate::protocol::{
    REPORT_LEN, Side, VENDOR_ID,
    led::{Color, CommandReply, Indicator, LedError, LedRequest},
};

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub side: Side,
    pub path: CString,
    pub name: String,
}

#[derive(Debug, Error)]
pub enum DeviceError {
    #[error(transparent)]
    Hid(#[from] HidError),
    #[error("multiple {0} panels found; select one with --{0}-path")]
    Ambiguous(Side),
    #[error("expected a {REPORT_LEN}-byte report, received {0} bytes")]
    ReportLength(usize),
}

/// Per-exchange deadline, chosen for the reference implementation rather than specified by firmware.
pub const LED_REPLY_TIMEOUT: Duration = Duration::from_millis(500);

/// Completion token scoped to one panel connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestId(u64);

#[derive(Debug, Error)]
pub enum RequestError {
    #[error(transparent)]
    Led(#[from] LedError),
    #[error(transparent)]
    Hid(#[from] HidError),
    #[error("an LED request is already pending")]
    Busy,
    #[error("{0:?} belongs to the other panel")]
    WrongSide(Indicator),
    #[error("wrote {0} bytes instead of the required 21")]
    ShortWrite(usize),
    #[error("LED reply timed out")]
    Timeout,
    #[error("LED command returned status {0:#04x}")]
    Status(u8),
    #[error("LED read returned non-Boolean state {0:#04x}")]
    State(u8),
}

#[derive(Debug)]
pub enum PanelEvent {
    Input([u8; REPORT_LEN]),
    /// Reads return Some(state); acknowledged writes return None.
    LedComplete {
        id: RequestId,
        result: Result<Option<bool>, RequestError>,
    },
    /// Intermediate acknowledgements and unmatched replies never enter the input decoder.
    Reply(CommandReply),
}

struct Pending {
    id: RequestId,
    request: LedRequest,
    next: Option<LedRequest>,
    sequence: u8,
    deadline: Instant,
}

pub struct Panel {
    pub info: DeviceInfo,
    device: HidDevice,
    sequence: u8,
    next_request: u64,
    pending: Option<Pending>,
}

impl Panel {
    pub fn read_led(&mut self, channel: u8) -> Result<RequestId, RequestError> {
        self.start(LedRequest::read(self.info.side, channel)?, None)
    }

    pub fn set_led(&mut self, channel: u8, enabled: bool) -> Result<RequestId, RequestError> {
        self.start(LedRequest::write(self.info.side, channel, enabled)?, None)
    }

    /// Disable the other color first. Multiple writes are acknowledged individually, not atomic.
    /// On failure, earlier writes remain in effect; no rollback is attempted.
    pub fn set_indicator_color(
        &mut self,
        indicator: Indicator,
        color: Color,
    ) -> Result<RequestId, RequestError> {
        if indicator.side() != self.info.side {
            return Err(RequestError::WrongSide(indicator));
        }
        let (red, green) = indicator.channels();
        let (first, next) = match (color, green) {
            (Color::Green, None) => return Err(LedError::Color(indicator).into()),
            (Color::Green, Some(green)) => ((red, false), Some((green, true))),
            (Color::Red, Some(green)) => ((green, false), Some((red, true))),
            (Color::Off, Some(green)) => ((red, false), Some((green, false))),
            (Color::Red, None) => ((red, true), None),
            (Color::Off, None) => ((red, false), None),
        };
        let request = LedRequest::write(self.info.side, first.0, first.1)?;
        let next = next
            .map(|(channel, enabled)| LedRequest::write(self.info.side, channel, enabled))
            .transpose()?;
        self.start(request, next)
    }

    fn send(&mut self, request: LedRequest) -> Result<(u8, Instant), RequestError> {
        self.sequence = self.sequence.wrapping_add(1);
        let mut bytes = [0; REPORT_LEN + 1];
        bytes[1..].copy_from_slice(&request.payload(self.sequence));
        let written = self.device.write(&bytes)?;
        if written != bytes.len() {
            return Err(RequestError::ShortWrite(written));
        }
        Ok((self.sequence, Instant::now() + LED_REPLY_TIMEOUT))
    }

    fn start(
        &mut self,
        request: LedRequest,
        next: Option<LedRequest>,
    ) -> Result<RequestId, RequestError> {
        if self.pending.is_some() {
            return Err(RequestError::Busy);
        }
        let (sequence, deadline) = self.send(request)?;
        let id = RequestId(self.next_request);
        self.next_request = self.next_request.wrapping_add(1);
        self.pending = Some(Pending {
            id,
            request,
            next,
            sequence,
            deadline,
        });
        Ok(id)
    }

    /// Poll without waiting for input or acknowledgements; call regularly to service deadlines.
    /// None means no report, not a disconnect or a released button.
    pub fn poll(&mut self) -> Result<Option<PanelEvent>, DeviceError> {
        if let Some(pending) = self.pending.take_if(|p| Instant::now() >= p.deadline) {
            return Ok(Some(PanelEvent::LedComplete {
                id: pending.id,
                result: Err(RequestError::Timeout),
            }));
        }
        let mut buffer = [0; REPORT_LEN + 1];
        let length = self.device.read_timeout(&mut buffer, 0)?;
        if length == 0 {
            return Ok(None);
        }
        let bytes: [u8; REPORT_LEN] = buffer[..length]
            .try_into()
            .map_err(|_| DeviceError::ReportLength(length))?;
        let Some(reply) = CommandReply::parse(&bytes) else {
            return Ok(Some(PanelEvent::Input(bytes)));
        };
        let Some(mut pending) = self
            .pending
            .take_if(|p| p.request.matches(reply, p.sequence))
        else {
            return Ok(Some(PanelEvent::Reply(reply)));
        };
        let result = if reply.status != 0 {
            Err(RequestError::Status(reply.status))
        } else if pending.request.is_read() {
            match reply.value {
                0 => Ok(Some(false)),
                1 => Ok(Some(true)),
                value => Err(RequestError::State(value)),
            }
        } else {
            Ok(None)
        };
        if result.is_err() || pending.next.is_none() {
            return Ok(Some(PanelEvent::LedComplete {
                id: pending.id,
                result,
            }));
        }
        let request = pending.next.take().unwrap();
        match self.send(request) {
            Ok((sequence, deadline)) => {
                self.pending = Some(Pending {
                    request,
                    sequence,
                    deadline,
                    ..pending
                });
                Ok(Some(PanelEvent::Reply(reply)))
            }
            Err(error) => Ok(Some(PanelEvent::LedComplete {
                id: pending.id,
                result: Err(error),
            })),
        }
    }
}

pub struct Usb {
    api: HidApi,
}

impl Usb {
    pub fn new() -> Result<Self, HidError> {
        Ok(Self {
            api: HidApi::new()?,
        })
    }

    pub fn refresh(&mut self) -> Result<Vec<DeviceInfo>, HidError> {
        self.api.refresh_devices()?;
        Ok(self
            .api
            .device_list()
            .filter_map(|info| {
                if info.vendor_id() != VENDOR_ID {
                    return None;
                }
                let side = Side::from_product_id(info.product_id())?;
                // Usage 0 is allowed for backends that cannot expose usage metadata.
                if !matches!(info.usage_page(), 0 | 0xff00) || !matches!(info.usage(), 0 | 1) {
                    return None;
                }
                Some(DeviceInfo {
                    side,
                    path: info.path().to_owned(),
                    name: info.product_string().unwrap_or("FEI HPC2").to_owned(),
                })
            })
            .collect())
    }

    pub fn open(
        &self,
        devices: &[DeviceInfo],
        side: Side,
        path: Option<&CStr>,
    ) -> Result<Option<Panel>, DeviceError> {
        let mut matches = devices
            .iter()
            .filter(|info| info.side == side && path.is_none_or(|path| info.path == path));
        let Some(info) = matches.next() else {
            return Ok(None);
        };
        if matches.next().is_some() {
            return Err(DeviceError::Ambiguous(side));
        }
        let device = self.api.open_path(&info.path)?;
        Ok(Some(Panel {
            info: info.clone(),
            device,
            sequence: 0,
            next_request: 0,
            pending: None,
        }))
    }
}
