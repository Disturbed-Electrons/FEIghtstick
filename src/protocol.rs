//! The 20-byte, unnumbered HPC2 input report. All offsets are zero-based.
use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const VENDOR_ID: u16 = 0x0f3f;
pub const REPORT_LEN: usize = 20;
pub const PRESSURE_REFERENCE: u8 = 15;
pub const JOYSTICK_REFERENCE: i32 = 90;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Left,
    Right,
}

impl Side {
    pub const fn product_id(self) -> u16 {
        match self {
            Self::Left => 0x0100,
            Self::Right => 0x0101,
        }
    }

    pub const fn from_product_id(id: u16) -> Option<Self> {
        match id {
            0x0100 => Some(Self::Left),
            0x0101 => Some(Self::Right),
            _ => None,
        }
    }

    pub const fn index(self) -> usize {
        match self {
            Self::Left => 0,
            Self::Right => 1,
        }
    }
}

impl fmt::Display for Side {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Left => "left",
            Self::Right => "right",
        })
    }
}

macro_rules! controls {
    ($( $variant:ident => ($id:literal, $side:ident, $encoding:expr) ),+ $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[repr(usize)]
        pub enum Control {
            $(#[serde(rename = $id)] $variant),+
        }

        impl Control {
            pub const ALL: [Self; 36] = [$(Self::$variant),+];
            pub const fn id(self) -> &'static str {
                match self { $(Self::$variant => $id),+ }
            }
            pub const fn side(self) -> Side {
                match self { $(Self::$variant => Side::$side),+ }
            }
            pub const fn encoding(self) -> Encoding {
                match self { $(Self::$variant => $encoding),+ }
            }
        }

        impl fmt::Display for Control {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(self.id()) }
        }

        impl FromStr for Control {
            type Err = String;
            fn from_str(id: &str) -> Result<Self, Self::Err> {
                match id { $($id => Ok(Self::$variant),)+ _ => Err(format!("unknown control: {id}")) }
            }
        }
    };
}

/// Signed motion is widened before normalization, so negating -128 is safe.
#[derive(Debug, Clone, Copy)]
pub enum Encoding {
    Button { byte: usize, mask: u8 },
    Pressure { byte: usize },
    Relative { byte: usize, sign: i16 },
    Joystick { byte: usize, sign: i16, axis: usize },
}

use Encoding::{Button, Joystick, Pressure, Relative};
controls! {
    LeftTrackballMinus => ("left.beamSpeedDown", Left, Button { byte: 2, mask: 0x01 }),
    LeftTrackballPlus => ("left.beamSpeedUp", Left, Button { byte: 2, mask: 0x02 }),
    L1 => ("left.L1", Left, Button { byte: 2, mask: 0x04 }),
    L2 => ("left.L2", Left, Button { byte: 2, mask: 0x08 }),
    L3 => ("left.L3", Left, Button { byte: 2, mask: 0x10 }),
    LeftMultifunctionFine => ("left.mfFine", Left, Button { byte: 2, mask: 0x20 }),
    LeftMultifunctionCoarse => ("left.mfCoarse", Left, Button { byte: 2, mask: 0x40 }),
    Exposure => ("left.exposure", Left, Button { byte: 2, mask: 0x80 }),
    Stigmator => ("left.stigmator", Left, Button { byte: 3, mask: 0x01 }),
    IntensityCoarse => ("left.intensityCoarse", Left, Button { byte: 3, mask: 0x02 }),
    IntensityFine => ("left.intensityFine", Left, Button { byte: 3, mask: 0x04 }),
    BetaUp => ("left.betaUp", Left, Pressure { byte: 4 }),
    BetaDown => ("left.betaDown", Left, Pressure { byte: 5 }),
    AlphaRight => ("left.alphaRight", Left, Pressure { byte: 6 }),
    AlphaLeft => ("left.alphaLeft", Left, Pressure { byte: 7 }),
    TrackballX => ("left.beamShift.X", Left, Relative { byte: 12, sign: 1 }),
    TrackballY => ("left.beamShift.Y", Left, Relative { byte: 13, sign: -1 }),
    Intensity => ("left.intensity", Left, Relative { byte: 14, sign: 1 }),
    MultifunctionX => ("left.multifunctionX", Left, Relative { byte: 17, sign: 1 }),
    StageSlower => ("right.stageSpeedDown", Right, Button { byte: 2, mask: 0x01 }),
    StageFaster => ("right.stageSpeedUp", Right, Button { byte: 2, mask: 0x02 }),
    DarkField => ("right.darkField", Right, Button { byte: 2, mask: 0x04 }),
    Diffraction => ("right.diffraction", Right, Button { byte: 2, mask: 0x08 }),
    Wobbler => ("right.wobbler", Right, Button { byte: 2, mask: 0x10 }),
    Eucentric => ("right.eucentric", Right, Button { byte: 2, mask: 0x20 }),
    R1 => ("right.R1", Right, Button { byte: 2, mask: 0x40 }),
    R2 => ("right.R2", Right, Button { byte: 2, mask: 0x80 }),
    R3 => ("right.R3", Right, Button { byte: 3, mask: 0x01 }),
    StageZDown => ("right.stageZDown", Right, Pressure { byte: 4 }),
    StageZUp => ("right.stageZUp", Right, Pressure { byte: 5 }),
    JoystickX => ("right.stagePosition.X", Right, Joystick { byte: 12, sign: 1, axis: 0 }),
    JoystickY => ("right.stagePosition.Y", Right, Joystick { byte: 13, sign: -1, axis: 1 }),
    Focus => ("right.focus", Right, Relative { byte: 14, sign: 1 }),
    FocusStep => ("right.focusStep", Right, Relative { byte: 15, sign: -1 }),
    Magnification => ("right.magnification", Right, Relative { byte: 16, sign: 1 }),
    MultifunctionY => ("right.multifunctionY", Right, Relative { byte: 17, sign: 1 }),
}

#[derive(Debug, Error)]
pub enum DecodeError {
    #[error("expected a {REPORT_LEN}-byte input payload, received {0} bytes")]
    Length(usize),
}

#[derive(Debug, Clone, Serialize)]
pub struct Reading {
    pub control: Control,
    /// Masked button bits, pressure byte, or signed wire delta before normalization.
    pub raw: i16,
    /// Button/pressure level, normalized relative delta, or estimated joystick position.
    /// A joystick position is unknown until explicitly centered, and after report loss.
    pub value: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Frame {
    pub side: Side,
    pub sequence: u8,
    pub sequence_gap: bool,
    pub readings: Vec<Reading>,
}

/// One decoder per physical panel. Idle time does not invalidate held controls.
#[derive(Debug)]
pub struct Decoder {
    side: Side,
    sequence: Option<u8>,
    joystick: Option<[i32; 2]>,
}

impl Decoder {
    pub fn new(side: Side) -> Self {
        Self {
            side,
            sequence: None,
            joystick: None,
        }
    }

    /// Declare the physical joystick centered now. Subsequent deltas estimate position.
    pub fn center(&mut self) {
        self.joystick = Some([0; 2]);
    }

    pub fn decode(&mut self, bytes: &[u8]) -> Result<Frame, DecodeError> {
        let bytes: &[u8; REPORT_LEN] = bytes
            .try_into()
            .map_err(|_| DecodeError::Length(bytes.len()))?;
        let sequence = bytes[1];
        let gap = self
            .sequence
            .is_some_and(|last| sequence != last.wrapping_add(1));
        self.sequence = Some(sequence);
        if gap {
            self.joystick = None;
        }
        let readings = Control::ALL
            .into_iter()
            .filter(|c| c.side() == self.side)
            .map(|control| {
                let (raw, value) = match control.encoding() {
                    Button { byte, mask } => {
                        let raw = i16::from(bytes[byte] & mask);
                        (raw, Some(i32::from(raw != 0)))
                    }
                    Pressure { byte } => (i16::from(bytes[byte]), Some(i32::from(bytes[byte]))),
                    Relative { byte, sign } => {
                        let raw = i16::from(bytes[byte] as i8);
                        (raw, Some(i32::from(raw * sign)))
                    }
                    Joystick { byte, sign, axis } => {
                        let raw = i16::from(bytes[byte] as i8);
                        let value = self.joystick.as_mut().map(|position| {
                            position[axis] = position[axis].saturating_add(i32::from(raw * sign));
                            position[axis]
                        });
                        (raw, value)
                    }
                };
                Reading {
                    control,
                    raw,
                    value,
                }
            })
            .collect();
        Ok(Frame {
            side: self.side,
            sequence,
            sequence_gap: gap,
            readings,
        })
    }
}
