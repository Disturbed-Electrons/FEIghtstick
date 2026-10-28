use vigem_client::{Client, TargetId, XGamepad, Xbox360Wired};

use super::{Output, OutputError};
use crate::mapping::GamepadState;

pub struct Gamepad {
    target: Xbox360Wired<Client>,
}

impl Gamepad {
    pub fn new() -> Result<Self, OutputError> {
        let client = Client::connect().map_err(|error| std::io::Error::other(format!(
            "cannot connect to ViGEmBus: {error}. Install the signed ViGEmBus prerequisite described in README.md."
        )))?;
        let mut target = Xbox360Wired::new(client, TargetId::XBOX360_WIRED);
        target.plugin()?;
        target.wait_ready()?;
        Ok(Self { target })
    }
}

impl Output for Gamepad {
    fn send(&mut self, state: &GamepadState) -> Result<(), OutputError> {
        // Shared button order -> XInput masks.
        const MASKS: [u16; 15] = [
            0x1000, 0x2000, 0x4000, 0x8000, 0x0100, 0x0200, 0x0020, 0x0010, 0x0400, 0x0040, 0x0080,
            0x0001, 0x0002, 0x0004, 0x0008,
        ];
        let buttons = MASKS.iter().enumerate().fold(0, |mask, (index, flag)| {
            if state.buttons & (1 << index) != 0 {
                mask | flag
            } else {
                mask
            }
        });
        let report = XGamepad {
            buttons: buttons.into(),
            left_trigger: (i32::from(state.axes[4]) * 255 / 32767) as u8,
            right_trigger: (i32::from(state.axes[5]) * 255 / 32767) as u8,
            thumb_lx: state.axes[0],
            thumb_ly: state.axes[1],
            thumb_rx: state.axes[2],
            thumb_ry: state.axes[3],
        };
        self.target.update(&report)?;
        Ok(())
    }
}

impl Drop for Gamepad {
    fn drop(&mut self) {
        let _ = self.send(&GamepadState::default());
        // The target's Drop implementation unplugs it from ViGEmBus.
    }
}
