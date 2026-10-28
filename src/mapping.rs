//! Application mapping is separate from the panel's wire protocol.
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::protocol::{Control, Encoding, Frame, JOYSTICK_REFERENCE, PRESSURE_REFERENCE, Side};

pub const DEFAULT_PROFILE: &str = include_str!("../profiles/default.toml");

/// Buttons follow the same order on every platform. Axes use right/up positive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct GamepadState {
    pub buttons: u16,
    /// Left X/Y, right X/Y, left trigger, right trigger. Triggers are 0..32767.
    pub axes: [i16; 6],
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Target {
    A,
    B,
    X,
    Y,
    LeftBumper,
    RightBumper,
    Back,
    Start,
    Guide,
    LeftThumb,
    RightThumb,
    DpadUp,
    DpadDown,
    DpadLeft,
    DpadRight,
    LeftX,
    LeftY,
    RightX,
    RightY,
    LeftTrigger,
    RightTrigger,
}

impl Target {
    pub const fn button(self) -> Option<usize> {
        let index = self as usize;
        if index < 15 { Some(index) } else { None }
    }

    pub const fn axis(self) -> Option<usize> {
        let index = self as usize;
        if index >= 15 { Some(index - 15) } else { None }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub control: Control,
    pub target: Target,
    /// Multiplies a normalized value; a negative scale reverses its direction.
    #[serde(default = "one")]
    pub scale: f32,
    #[serde(default = "threshold")]
    pub threshold: f32,
}

fn one() -> f32 {
    1.0
}
fn threshold() -> f32 {
    0.5
}
fn pulse_ms() -> u64 {
    120
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    #[serde(default = "pulse_ms")]
    pub motion_hold_ms: u64,
    pub bindings: Vec<Binding>,
}

#[derive(Debug, Error)]
pub enum ProfileError {
    #[error(transparent)]
    Toml(#[from] toml::de::Error),
    #[error("motion_hold_ms must be between 1 and 1000")]
    HoldTime,
    #[error(
        "invalid scale or threshold for {0}; scale must be finite and threshold must be in (0, 1]"
    )]
    Binding(Control),
}

impl Profile {
    pub fn parse(text: &str) -> Result<Self, ProfileError> {
        let profile: Self = toml::from_str(text)?;
        if !(1..=1000).contains(&profile.motion_hold_ms) {
            return Err(ProfileError::HoldTime);
        }
        for binding in &profile.bindings {
            if !binding.scale.is_finite() || !(0.0 < binding.threshold && binding.threshold <= 1.0)
            {
                return Err(ProfileError::Binding(binding.control));
            }
        }
        Ok(profile)
    }
}

pub struct Mapper {
    profile: Profile,
    values: [Option<f32>; 36],
    motion_at: [Option<Instant>; 36],
}

impl Mapper {
    pub fn new(profile: Profile) -> Self {
        Self {
            profile,
            values: [None; 36],
            motion_at: [None; 36],
        }
    }

    pub fn disconnect(&mut self, side: Side) {
        for control in Control::ALL.into_iter().filter(|c| c.side() == side) {
            self.values[control as usize] = None;
            self.motion_at[control as usize] = None;
        }
    }

    pub fn center(&mut self) {
        self.values[Control::JoystickX as usize] = Some(0.0);
        self.values[Control::JoystickY as usize] = Some(0.0);
    }

    pub fn update(&mut self, frame: &Frame, now: Instant) {
        if frame.sequence_gap {
            self.disconnect(frame.side);
        }
        for reading in &frame.readings {
            let index = reading.control as usize;
            let divisor = match reading.control.encoding() {
                Encoding::Button { .. } => 1.0,
                Encoding::Pressure { .. } => f32::from(PRESSURE_REFERENCE),
                Encoding::Relative { .. } => {
                    // Zero relative increments do not stop an active motion pulse.
                    if reading.value == Some(0) {
                        continue;
                    }
                    self.motion_at[index] = Some(now);
                    16.0
                }
                Encoding::Joystick { .. } => JOYSTICK_REFERENCE as f32,
            };
            self.values[index] = reading.value.map(|value| value as f32 / divisor);
        }
    }

    pub fn state(&self, now: Instant) -> GamepadState {
        let mut state = GamepadState::default();
        let mut axes = [0.0f32; 6];
        let hold = Duration::from_millis(self.profile.motion_hold_ms);
        for binding in &self.profile.bindings {
            let index = binding.control as usize;
            if self.motion_at[index].is_some_and(|time| now.duration_since(time) >= hold) {
                continue;
            }
            let value = self.values[index].unwrap_or(0.0) * binding.scale;
            if let Some(button) = binding.target.button() {
                if value >= binding.threshold {
                    state.buttons |= 1 << button;
                }
            } else if let Some(axis) = binding.target.axis() {
                axes[axis] += value;
            }
        }
        for (index, value) in axes.into_iter().enumerate() {
            let min = if index >= 4 { 0.0 } else { -1.0 };
            state.axes[index] = (value.clamp(min, 1.0) * f32::from(i16::MAX)).round() as i16;
        }
        state
    }
}
