//! Native FEI HPC2 controls, lighting, and device information with configurable gamepad mapping.
//!
//! The protocol module decodes report data; the mapping module converts readings to gamepad state.
pub mod device;
pub mod mapping;
pub mod output;
pub mod protocol;
