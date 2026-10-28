//! Synchronous IOHIDUserDevice calls. This module owns the sole native device reference.
use std::{ffi::c_void, io, ptr::NonNull};

use core_foundation::{
    base::{CFRelease, CFTypeRef, TCFType},
    data::CFData,
    dictionary::{CFDictionary, CFDictionaryRef},
    number::CFNumber,
    string::CFString,
};

use super::{Output, OutputError};
use crate::mapping::GamepadState;

// Game pad: 15 buttons + padding, four signed 16-bit stick axes, two unsigned triggers.
const DESCRIPTOR: &[u8] = &[
    0x05, 0x01, 0x09, 0x05, 0xa1, 0x01, 0x05, 0x09, 0x19, 0x01, 0x29, 0x0f, 0x15, 0x00, 0x25, 0x01,
    0x75, 0x01, 0x95, 0x0f, 0x81, 0x02, 0x75, 0x01, 0x95, 0x01, 0x81, 0x03, 0x05, 0x01, 0x09, 0x30,
    0x09, 0x31, 0x09, 0x33, 0x09, 0x34, 0x16, 0x01, 0x80, 0x26, 0xff, 0x7f, 0x75, 0x10, 0x95, 0x04,
    0x81, 0x02, 0x09, 0x32, 0x09, 0x35, 0x15, 0x00, 0x26, 0xff, 0x00, 0x75, 0x08, 0x95, 0x02, 0x81,
    0x02, 0xc0,
];

#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IOHIDUserDeviceCreate(
        allocator: *const c_void,
        properties: CFDictionaryRef,
        options: u32,
    ) -> *mut c_void;
    fn IOHIDUserDeviceHandleReport(device: *mut c_void, report: *const u8, length: isize) -> i32;
}

pub struct Gamepad {
    device: NonNull<c_void>,
}

impl Gamepad {
    pub fn new() -> Result<Self, OutputError> {
        let properties = CFDictionary::from_CFType_pairs(&[
            (
                CFString::new("ReportDescriptor").as_CFType(),
                CFData::from_buffer(DESCRIPTOR).as_CFType(),
            ),
            (
                CFString::new("Product").as_CFType(),
                CFString::new("FEIghtstick").as_CFType(),
            ),
            (
                CFString::new("Manufacturer").as_CFType(),
                CFString::new("FEIghtstick").as_CFType(),
            ),
            (
                CFString::new("PrimaryUsagePage").as_CFType(),
                CFNumber::from(1i32).as_CFType(),
            ),
            (
                CFString::new("PrimaryUsage").as_CFType(),
                CFNumber::from(5i32).as_CFType(),
            ),
        ]);
        // SAFETY: properties contains CFString keys and correctly typed, live CF values.
        // Create returns an owned reference, released exactly once in Drop.
        let device =
            unsafe { IOHIDUserDeviceCreate(std::ptr::null(), properties.as_concrete_TypeRef(), 0) };
        let device = NonNull::new(device).ok_or_else(|| io::Error::new(
            io::ErrorKind::PermissionDenied,
            "macOS refused the virtual gamepad. Use a release signed with an Apple-approved com.apple.developer.hid.virtual.device entitlement and matching provisioning profile; an ordinary cargo build cannot grant it.",
        ))?;
        Ok(Self { device })
    }
}

impl Output for Gamepad {
    fn send(&mut self, state: &GamepadState) -> Result<(), OutputError> {
        let mut report = [0u8; 12];
        report[..2].copy_from_slice(&state.buttons.to_le_bytes());
        for index in 0..4 {
            // Generic HID Y points down; our shared state points up.
            let value = if index % 2 == 1 {
                state.axes[index].saturating_neg()
            } else {
                state.axes[index]
            };
            report[2 + index * 2..4 + index * 2].copy_from_slice(&value.to_le_bytes());
        }
        for index in 0..2 {
            report[10 + index] = (i32::from(state.axes[4 + index]) * 255 / 32767) as u8;
        }
        // SAFETY: device is owned and live; report remains valid for this synchronous call.
        let result = unsafe {
            IOHIDUserDeviceHandleReport(
                self.device.as_ptr(),
                report.as_ptr(),
                report.len() as isize,
            )
        };
        if result != 0 {
            return Err(io::Error::other(format!(
                "IOHIDUserDeviceHandleReport failed: {result:#010x}"
            ))
            .into());
        }
        Ok(())
    }
}

impl Drop for Gamepad {
    fn drop(&mut self) {
        let _ = self.send(&GamepadState::default());
        // SAFETY: this is the owned Create-rule reference, no longer used after release.
        unsafe { CFRelease(self.device.as_ptr() as CFTypeRef) };
    }
}
