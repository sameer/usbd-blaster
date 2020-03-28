#![no_std]
#![no_main]

extern crate arduino_mkrvidor4000 as hal;

use cortex_m::peripheral::syst::SystClkSource;
use hal::clock::GenericClockController;
use hal::entry;
use hal::gpio::IntoFunction;
use hal::pac::{
    gclk::{clkctrl::GEN_A, genctrl::SRC_A},
    interrupt, CorePeripherals, Peripherals, NVIC,
};
use hal::prelude::*;
use hal::usb::usb_device::{bus::UsbBusAllocator, prelude::*};
use hal::usb::UsbBus;

mod blaster;

// #[link_section = "FLASH_FPGA"]
// const FLASH_FPGA: [u8; 2 * 1024 * 1024] = [0u8; 2 * 1024 * 1024];

static mut USB_ALLOCATOR: Option<UsbBusAllocator<UsbBus>> = None;
static mut USB_BLASTER: Option<blaster::USBBlaster<UsbBus>> = None;
static mut USB_BUS: Option<UsbDevice<UsbBus>> = None;
static mut LED: Option<hal::gpio::Pb8<hal::gpio::Output<hal::gpio::OpenDrain>>> = None;

#[entry]
fn main() -> ! {
    let mut peripherals = Peripherals::take().unwrap();
    let mut core = CorePeripherals::take().unwrap();
    let mut clocks = GenericClockController::with_internal_32kosc(
        peripherals.GCLK,
        &mut peripherals.PM,
        &mut peripherals.SYSCTRL,
        &mut peripherals.NVMCTRL,
    );
    // FPGA Clock
    clocks
        .configure_gclk_divider_and_source(GEN_A::GCLK2, 1, SRC_A::DFLL48M, true)
        .unwrap();
    // USB Clock
    let usb_gclk = clocks
        .configure_gclk_divider_and_source(GEN_A::GCLK6, 1, SRC_A::DFLL48M, true)
        .unwrap();
    let usb_clock = clocks.usb(&usb_gclk).unwrap();
    core.SYST.set_clock_source(SystClkSource::Core);

    let mut pins = hal::Pins::new(peripherals.PORT);

    let allocator = unsafe {
        USB_ALLOCATOR = UsbBusAllocator::new(UsbBus::new(
            &usb_clock,
            &mut peripherals.PM,
            pins.usb_n.into_function(&mut pins.port),
            pins.usb_p.into_function(&mut pins.port),
            peripherals.USB,
        ))
        .into();
        USB_ALLOCATOR.as_ref().unwrap()
    };
    unsafe {
        LED = pins
            .led_builtin
            .into_open_drain_output(&mut pins.port)
            .into();
        USB_BLASTER = blaster::USBBlaster::new(
            USB_ALLOCATOR.as_ref().unwrap(),
            pins.fpga_tdi.into_push_pull_output(&mut pins.port),
            pins.fpga_tck.into_push_pull_output(&mut pins.port),
            pins.fpga_tms.into_push_pull_output(&mut pins.port),
            pins.fpga_tdo,
        )
        .into();
        USB_BUS = UsbDeviceBuilder::new(&allocator, blaster::ALTERA_BLASTER_USB_VID_PID)
            .manufacturer("Arduino LLC")
            .product("Arduino MKR Vidor 4000")
            .serial_number("12345678")
            .device_release(0x0400)
            .max_power(500)
            .build()
            .into();
        core.NVIC.set_priority(interrupt::USB, 1);
        NVIC::unmask(interrupt::USB);
    }

    loop {
        // cortex_m::interrupt::free(|_| unsafe {
        //     USB_BLASTER.as_mut().map(|blaster| {
        //     });
        // });
    }
}

#[interrupt]
fn USB() {
    unsafe {
        USB_BUS.as_mut().map(|usb_dev| {
            USB_BLASTER.as_mut().map(|blaster| {
                usb_dev.poll(&mut [blaster]);
                blaster.read().ok();
                blaster.handle();
                blaster.write(false).ok();
            });
        });
    };
}
