#![no_std]
#![no_main]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

extern crate arduino_mkrvidor4000 as hal;

use crate::hal::gpio::IntoFunction;
use hal::clock::GenericClockController;
use hal::delay::Delay;
use hal::entry;
use hal::pac::{CorePeripherals, Peripherals};
use hal::prelude::*;
use hal::usb::{UsbBus};
use hal::usb::usb_device::bus::UsbBusAllocator;
use hal::pac::gclk::genctrl::SRC_A;
use hal::pac::gclk::clkctrl::GEN_A;

// use usb_device::prelude::*;


// mod ft245rom;

#[entry]
fn main() -> ! {
    let mut peripherals = Peripherals::take().unwrap();
    let core = CorePeripherals::take().unwrap();
    let mut clocks = GenericClockController::with_internal_32kosc(
        peripherals.GCLK,
        &mut peripherals.PM,
        &mut peripherals.SYSCTRL,
        &mut peripherals.NVMCTRL,
    );

    // let ft245 = UsbDeviceBuilder::new(bus.allocator());
    let mut pins = hal::Pins::new(peripherals.PORT);

    clocks.configure_gclk_divider_and_source(GEN_A::GCLK2, 1, SRC_A::DFLL48M, false);
    let usb_gclk = clocks.get_gclk(GEN_A::GCLK2).unwrap();
    let usb_clock = &clocks.usb(&usb_gclk).unwrap();

    let allocator = UsbBusAllocator::new(UsbBus::new(
        usb_clock,
        &mut peripherals.PM,
        pins.usb_n.into_function(&mut pins.port),
        pins.usb_p.into_function(&mut pins.port),
        peripherals.USB,
    ));

    // unsafe {
    //     USB_SERIAL = Some(SerialPort::new(&bus_allocator));
    //     USB_BUS = Some(
    //         UsbDeviceBuilder::new(&bus_allocator, UsbVidPid(0x16c0, 0x27dd))
    //             .manufacturer("Fake company")
    //             .product("Serial port")
    //             .serial_number("TEST")
    //             .device_class(USB_CLASS_CDC)
    //             .build(),
    //     );
    // }

    let mut led = pins.led_builtin.into_open_drain_output(&mut pins.port);
    let mut delay = Delay::new(core.SYST, &mut clocks);

    loop {
        delay.delay_ms(200u8);
        led.set_high().unwrap();
        delay.delay_ms(200u8);
        led.set_low().unwrap();
    }
}
