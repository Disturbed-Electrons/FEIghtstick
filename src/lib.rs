//! Native FEI HPC2 controls, LEDs, and label illumination with configurable gamepad mapping.
//!
//! The protocol and mapping modules work without USB hardware or a virtual device.
pub mod device;
pub mod mapping;
pub mod output;
pub mod protocol;
