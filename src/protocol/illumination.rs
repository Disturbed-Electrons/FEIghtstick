//! Panel label illumination commands.
use super::REPORT_LEN;

#[derive(Debug, Clone, Copy)]
pub enum IlluminationRequest {
    Read,
    Write(bool),
}

impl IlluminationRequest {
    pub(crate) const fn operation(self) -> u8 {
        match self {
            Self::Read => 0x05,
            Self::Write(_) => 0x06,
        }
    }

    /// An unnumbered payload; HIDAPI requires an additional leading zero.
    pub fn payload(self, sequence: u8) -> [u8; REPORT_LEN] {
        let mut bytes = [0; REPORT_LEN];
        bytes[0] = self.operation();
        if let Self::Write(enabled) = self {
            bytes[2] = u8::from(enabled);
        }
        bytes[19] = sequence;
        bytes
    }
}
