//! The backend is selected at compile time, without platform-specific user configuration.
use crate::mapping::GamepadState;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
pub use linux::Gamepad;
#[cfg(target_os = "macos")]
pub use macos::Gamepad;
#[cfg(target_os = "windows")]
pub use windows::Gamepad;

pub type OutputError = Box<dyn std::error::Error>;

/// A virtual controller is neutralized on orderly shutdown before its device is removed.
pub trait Output {
    fn send(&mut self, state: &GamepadState) -> Result<(), OutputError>;
}
