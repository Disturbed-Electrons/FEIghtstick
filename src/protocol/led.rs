//! HPC2 indicator channels and LED commands.
use thiserror::Error;

use super::{REPORT_LEN, Side};

pub const LEFT_CHANNELS: &[u8] = &[0, 1, 2, 3, 4, 5, 6, 8, 9, 10, 11, 12, 13, 14];
pub const RIGHT_CHANNELS: &[u8] = &[2, 3, 4, 5, 6, 11, 12, 13, 14];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Off,
    Red,
    Green,
}

/// Named panel indicators with channel mappings for each color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Indicator {
    Coarse,
    Fine,
    L1,
    L2,
    L3,
    Exposure,
    Stigmator,
    R1,
    R2,
    R3,
    Wobbler,
    DarkField,
    Diffraction,
}

impl Indicator {
    pub const fn side(self) -> Side {
        match self {
            Self::Coarse
            | Self::Fine
            | Self::L1
            | Self::L2
            | Self::L3
            | Self::Exposure
            | Self::Stigmator => Side::Left,
            Self::R1
            | Self::R2
            | Self::R3
            | Self::Wobbler
            | Self::DarkField
            | Self::Diffraction => Side::Right,
        }
    }

    /// Red channel and optional green channel for this indicator.
    pub const fn channels(self) -> (u8, Option<u8>) {
        match self {
            Self::Coarse => (0, None),
            Self::Fine => (8, None),
            Self::L1 => (11, Some(3)),
            Self::L2 => (10, Some(2)),
            Self::L3 => (9, Some(1)),
            Self::Exposure => (6, None),
            Self::Stigmator => (14, None),
            Self::R1 => (13, Some(4)),
            Self::R2 => (12, Some(3)),
            Self::R3 => (11, Some(2)),
            Self::Wobbler => (5, None),
            Self::DarkField => (6, None),
            Self::Diffraction => (14, None),
        }
    }
}

#[derive(Debug, Error)]
pub enum LedError {
    #[error("channel {channel} is not assigned on the {side} panel")]
    Channel { side: Side, channel: u8 },
    #[error("{0:?} has no observed green channel")]
    Color(Indicator),
}

/// An unnumbered 20-byte output payload; HIDAPI requires an additional leading zero.
#[derive(Debug, Clone, Copy)]
pub struct LedRequest {
    operation: u8,
    channel: u8,
    value: u8,
}

impl LedRequest {
    pub const fn read_states() -> Self {
        Self {
            operation: 0x09,
            channel: 0,
            value: 0,
        }
    }

    pub fn read(side: Side, channel: u8) -> Result<Self, LedError> {
        let channels = match side {
            Side::Left => LEFT_CHANNELS,
            Side::Right => RIGHT_CHANNELS,
        };
        if !channels.contains(&channel) {
            return Err(LedError::Channel { side, channel });
        }
        Ok(Self {
            operation: 0x07,
            channel,
            value: 0,
        })
    }

    pub fn write(side: Side, channel: u8, enabled: bool) -> Result<Self, LedError> {
        Ok(Self {
            operation: 0x08,
            value: u8::from(enabled),
            ..Self::read(side, channel)?
        })
    }

    pub fn payload(self, sequence: u8) -> [u8; REPORT_LEN] {
        let mut bytes = [0; REPORT_LEN];
        bytes[0] = self.operation;
        bytes[2] = self.channel;
        bytes[3] = self.value;
        bytes[19] = sequence;
        bytes
    }

    pub(crate) fn matches(self, reply: CommandReply, sequence: u8) -> bool {
        reply.operation == self.operation
            && reply.sequence == sequence
            && (self.operation == 0x09 || reply.channel == self.channel)
    }

    pub(crate) const fn is_read(self) -> bool {
        self.operation == 0x07
    }
}

/// A command reply with `channel`, `value`, and `sequence` interpreted using the `07`/`08` LED layout.
/// The LED request sequence at byte 19 is separate from the control-report sequence at byte 1.
#[derive(Debug, Clone, Copy)]
pub struct CommandReply {
    pub operation: u8,
    pub status: u8,
    pub channel: u8,
    pub value: u8,
    pub sequence: u8,
}

impl CommandReply {
    pub fn parse(bytes: &[u8; REPORT_LEN]) -> Option<Self> {
        (bytes[0] != 0).then_some(Self {
            operation: bytes[0],
            status: bytes[1],
            channel: bytes[2],
            value: bytes[3],
            sequence: bytes[19],
        })
    }
}
