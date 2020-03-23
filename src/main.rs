#![no_std]
#![no_main]

extern crate arduino_mkrvidor4000 as hal;

use hal::clock::GenericClockController;
use hal::delay::Delay;
use hal::entry;
use hal::gpio::IntoFunction;
use hal::pac::{
    gclk::{clkctrl::GEN_A, genctrl::SRC_A},
    interrupt, CorePeripherals, Peripherals, NVIC,
};
use hal::prelude::*;
use hal::usb::usb_device::{bus::UsbBusAllocator, prelude::*};
use hal::usb::UsbBus;
use usbd_serial::{SerialPort, USB_CLASS_CDC};

mod blaster;



static mut USB_ALLOCATOR: Option<UsbBusAllocator<UsbBus>> = None;
// static mut USB_SERIAL: Option<SerialPort<UsbBus>> = None;
static mut USB_BLASTER: Option<blaster::Blaster<UsbBus>> = None;
static mut USB_BUS: Option<UsbDevice<UsbBus>> = None;

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

    // let ft245 = UsbDeviceBuilder::new(bus.allocator());
    let mut pins = hal::Pins::new(peripherals.PORT);

    let usb_gclk = clocks
        .configure_gclk_divider_and_source(GEN_A::GCLK6, 1, SRC_A::DFLL48M, true)
        .unwrap();
    let usb_clock = &clocks.usb(&usb_gclk).unwrap();

    let allocator = unsafe {
        USB_ALLOCATOR = UsbBusAllocator::new(UsbBus::new(
            usb_clock,
            &mut peripherals.PM,
            pins.usb_n.into_function(&mut pins.port),
            pins.usb_p.into_function(&mut pins.port),
            peripherals.USB,
        ))
        .into();
        USB_ALLOCATOR.as_ref().unwrap()
    };
    unsafe {
        USB_BLASTER = blaster::Blaster::new(
            USB_ALLOCATOR.as_ref().unwrap(),
            pins.fpga_tdi,
            pins.fpga_tck.into_push_pull_output(&mut pins.port),
            pins.fpga_tms.into_push_pull_output(&mut pins.port),
            pins.fpga_tdo.into_push_pull_output(&mut pins.port),
        )
        .into();
        // USB_SERIAL = SerialPort::new(&allocator).into();
        USB_BUS = UsbDeviceBuilder::new(&allocator, blaster::ALTERA_BLASTER_USB_VID_PID)
            .manufacturer("Arduino LLC")
            .product("Arduino MKR Vidor 4000")
            .serial_number("1234")
            // .device_class(USB_CLASS_CDC)
            // .device_release(0x0004)
            // .max_packet_size_0(16) // setOutEpSize (?)
            .max_power(500)
            .build()
            .into();
        core.NVIC.set_priority(interrupt::USB, 1);
        NVIC::unmask(interrupt::USB);
    }
    let mut led = pins.led_builtin.into_open_drain_output(&mut pins.port);
    let mut delay = Delay::new(core.SYST, &mut clocks);

    loop {
        cortex_m::interrupt::free(|_| unsafe {
            // if HIGH {
            //     led.set_high().unwrap();
            // } else {
            //     led.set_low().unwrap();
            // }
        });
    }
}

static mut HIGH: bool = false;

#[interrupt]
fn USB() {
    unsafe {
        USB_BUS.as_mut().map(|usb_dev| {
            USB_BLASTER.as_mut().map(|blaster| {
                usb_dev.poll(&mut [blaster]);
            });
            // USB_SERIAL.as_mut().map(|serial| {
            //     usb_dev.poll(&mut [serial]);
            //     let mut buf = [0u8; 64];

            //     if let Ok(count) = serial.read(&mut buf) {
            //         for (i, c) in buf.iter().enumerate() {
            //             if i > count {
            //                 break;
            //             }
            //             match c.clone() as char {
            //                 'H' => {
            //                     HIGH = true;
            //                 }
            //                 'L' => {
            //                     HIGH = false;
            //                 }
            //                 _ => {}
            //             }
            //         }
            //     };
            // });
        });
    };
}
