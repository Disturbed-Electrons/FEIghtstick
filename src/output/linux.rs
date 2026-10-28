use evdev::{
    AbsInfo, AbsoluteAxisCode, AttributeSet, EventType, InputEvent, KeyCode, UinputAbsSetup,
    uinput::VirtualDevice,
};

use super::{Output, OutputError};
use crate::mapping::GamepadState;

const BUTTONS: [KeyCode; 15] = [
    KeyCode::BTN_SOUTH,
    KeyCode::BTN_EAST,
    KeyCode::BTN_WEST,
    KeyCode::BTN_NORTH,
    KeyCode::BTN_TL,
    KeyCode::BTN_TR,
    KeyCode::BTN_SELECT,
    KeyCode::BTN_START,
    KeyCode::BTN_MODE,
    KeyCode::BTN_THUMBL,
    KeyCode::BTN_THUMBR,
    KeyCode::BTN_DPAD_UP,
    KeyCode::BTN_DPAD_DOWN,
    KeyCode::BTN_DPAD_LEFT,
    KeyCode::BTN_DPAD_RIGHT,
];
const AXES: [AbsoluteAxisCode; 6] = [
    AbsoluteAxisCode::ABS_X,
    AbsoluteAxisCode::ABS_Y,
    AbsoluteAxisCode::ABS_RX,
    AbsoluteAxisCode::ABS_RY,
    AbsoluteAxisCode::ABS_Z,
    AbsoluteAxisCode::ABS_RZ,
];

pub struct Gamepad {
    device: VirtualDevice,
}

impl Gamepad {
    pub fn new() -> Result<Self, OutputError> {
        let keys: AttributeSet<KeyCode> = BUTTONS.into_iter().collect();
        let mut builder = VirtualDevice::builder()?
            .name("FEIghtstick")
            .with_keys(&keys)?;
        for (index, axis) in AXES.into_iter().enumerate() {
            let min = if index >= 4 { 0 } else { -32767 };
            builder = builder.with_absolute_axis(&UinputAbsSetup::new(
                axis,
                AbsInfo::new(0, min, 32767, 0, 0, 0),
            ))?;
        }
        let device = builder.build()?;
        Ok(Self { device })
    }
}

impl Output for Gamepad {
    fn send(&mut self, state: &GamepadState) -> Result<(), OutputError> {
        let mut events = Vec::with_capacity(21);
        for (index, button) in BUTTONS.into_iter().enumerate() {
            events.push(InputEvent::new(
                EventType::KEY.0,
                button.0,
                i32::from(state.buttons & (1 << index) != 0),
            ));
        }
        for (index, axis) in AXES.into_iter().enumerate() {
            let value = i32::from(state.axes[index]);
            let value = if index == 1 || index == 3 {
                -value
            } else {
                value
            };
            events.push(InputEvent::new(EventType::ABSOLUTE.0, axis.0, value));
        }
        self.device.emit(&events)?;
        Ok(())
    }
}

impl Drop for Gamepad {
    fn drop(&mut self) {
        let _ = self.send(&GamepadState::default());
    }
}
