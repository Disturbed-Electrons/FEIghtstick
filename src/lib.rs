//! Native FEI HPC2 input: USB transport, protocol decoding, and configurable gamepad mapping.
//!
//! The protocol and mapping modules work without USB hardware or a virtual device.
pub mod device;
pub mod mapping;
pub mod output;
pub mod protocol;
