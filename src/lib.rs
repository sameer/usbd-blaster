#![no_std]
#![forbid(unsafe_code)]

mod blaster;
mod class;
mod ft245;
mod port;

use usb_device::prelude::UsbVidPid;

extern crate embedded_hal as hal;

pub const ALTERA_BLASTER_USB_VID_PID: UsbVidPid = UsbVidPid(0x09FB, 0x6001);
pub use blaster::Blaster;
