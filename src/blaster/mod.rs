use hal::gpio::IntoFunction;
use hal::prelude::*;
use hal::usb::usb_device::{class_prelude::*, control::RequestType, descriptor, Result};
use hal::Pins;

mod class;
mod ft245;

use class::BlasterClass;

pub struct Blaster<'a, B: UsbBus> {
    class: BlasterClass<'a, B>,
    tdi: hal::gpio::Pa12<hal::gpio::Input<hal::gpio::Floating>>,
    tck: hal::gpio::Pa13<hal::gpio::Output<hal::gpio::PushPull>>,
    tms: hal::gpio::Pa14<hal::gpio::Output<hal::gpio::PushPull>>,
    tdo: hal::gpio::Pa15<hal::gpio::Output<hal::gpio::PushPull>>,
}

impl<'a, B: UsbBus> Blaster<'a, B> {
    pub fn new(
        alloc: &'a UsbBusAllocator<B>,
        tdi: hal::gpio::Pa12<hal::gpio::Input<hal::gpio::Floating>>,
        tck: hal::gpio::Pa13<hal::gpio::Output<hal::gpio::PushPull>>,
        tms: hal::gpio::Pa14<hal::gpio::Output<hal::gpio::PushPull>>,
        tdo: hal::gpio::Pa15<hal::gpio::Output<hal::gpio::PushPull>>,
    ) -> Blaster<'a, B> {
        Blaster {
            class: BlasterClass::new(alloc, 64),
            tdi,
            tck,
            tms,
            tdo,
        }
    }
}

