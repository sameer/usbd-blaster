#![no_std]
#![forbid(unsafe_code)]

mod blaster;
mod class;
mod ft245;
mod port;

use usb_device::prelude::UsbVidPid;

extern crate embedded_hal as hal;

/// The Vendor ID and Product ID for an Altera Blaster.
/// Use this when building your USB device for Quartus to recognize the blaster.
pub const ALTERA_BLASTER_USB_VID_PID: UsbVidPid = UsbVidPid(0x09FB, 0x6001);

pub use blaster::Blaster;
