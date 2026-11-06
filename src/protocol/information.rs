//! Panel information commands and reply data.
use super::REPORT_LEN;

#[derive(Debug, Clone, Copy)]
pub struct InformationRequest;

impl InformationRequest {
    pub(crate) const OPERATION: u8 = 0x02;

    /// An unnumbered payload; HIDAPI requires an additional leading zero.
    pub fn payload(self, sequence: u8) -> [u8; REPORT_LEN] {
        let mut bytes = [0; REPORT_LEN];
        bytes[0] = Self::OPERATION;
        bytes[19] = sequence;
        bytes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PanelInformation {
    /// Payload bytes 2–18, preserved in wire order.
    pub data: [u8; 17],
}

impl PanelInformation {
    pub(crate) fn from_reply(bytes: &[u8; REPORT_LEN]) -> Self {
        let mut data = [0; 17];
        data.copy_from_slice(&bytes[2..19]);
        Self { data }
    }

    pub const fn version_major(&self) -> u8 {
        self.data[4]
    }

    pub const fn version_minor(&self) -> u8 {
        self.data[3]
    }
}
