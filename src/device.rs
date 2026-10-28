//! Nonexclusive HID input with one connection per panel side.
use std::ffi::{CStr, CString};

use hidapi::{HidApi, HidDevice, HidError};
use thiserror::Error;

use crate::protocol::{REPORT_LEN, Side, VENDOR_ID};

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
}

pub struct Panel {
    pub info: DeviceInfo,
    device: HidDevice,
}

impl Panel {
    /// None means no new input, not a disconnect or a released button.
    pub fn read(&self, buffer: &mut [u8; REPORT_LEN + 1]) -> Result<Option<usize>, HidError> {
        match self.device.read_timeout(buffer, 0)? {
            0 => Ok(None),
            length => Ok(Some(length)),
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
        }))
    }
}
