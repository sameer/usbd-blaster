use hal::digital::v2::{InputPin, OutputPin};
use usb_device::{class_prelude::*, control::RequestType};

use crate::class::{BlasterClass, FTDI_MODEM_STA_DUMMY, INTERFACE_A_INDEX};
use crate::port::Port;

/// Depending on the underlying USB library (libusb or similar) the OS may send/receive more bytes than declared in the USB endpoint
/// If this happens to you, please open an issue for this crate on GitHub.
const BLASTER_WRITE_SIZE: usize = 64;
const BLASTER_READ_SIZE: usize = 64;

/// Blaster device class
pub struct Blaster<
    'a,
    B: UsbBus,
    E,
    TDI: OutputPin<Error = E>,
    TCK: OutputPin<Error = E>,
    TMS: OutputPin<Error = E>,
    TDO: InputPin<Error = E>,
> {
    class: BlasterClass<'a, B>,
    port: Port<E, TDI, TCK, TMS, TDO>,
    send_buffer: [u8; BLASTER_WRITE_SIZE],
    send_len: usize,
    recv_buffer: [u8; BLASTER_READ_SIZE],
    recv_len: usize,
}

impl<
        'a,
        B: UsbBus,
        E,
        TDI: OutputPin<Error = E>,
        TCK: OutputPin<Error = E>,
        TMS: OutputPin<Error = E>,
        TDO: InputPin<Error = E>,
    > Blaster<'a, B, E, TDI, TCK, TMS, TDO>
{
    /// Allocate a Blaster on the USB bus. Takes control of the four JTAG pins.
    /// The JTAG pins can be any pins you want, just make sure you assign them correctly.
    pub fn new(
        alloc: &'a UsbBusAllocator<B>,
        tdi: TDI,
        tck: TCK,
        tms: TMS,
        tdo: TDO,
    ) -> Blaster<'a, B, E, TDI, TCK, TMS, TDO> {
        Blaster {
            class: BlasterClass::new(alloc, BLASTER_WRITE_SIZE as u16, BLASTER_READ_SIZE as u16),
            port: Port::new(tdi, tck, tms, tdo),
            send_buffer: [0u8; BLASTER_WRITE_SIZE],
            send_len: 0,
            recv_buffer: [0u8; BLASTER_READ_SIZE],
            recv_len: 0,
        }
    }

    /// Read data from the host output endpoint into the Blaster's internal read buffer.
    pub fn read(&mut self) -> usb_device::Result<usize> {
        if self.recv_len == self.recv_buffer.len() {
            return Err(UsbError::WouldBlock);
        }
        let amount = self.class.read(&mut self.recv_buffer[self.recv_len..])?;
        self.recv_len += amount;
        Ok(amount)
    }

    /// Write data to the host input endpoint from the Blaster's internal write buffer.
    /// The heartbeat parameter must be true at least once every 10 milliseconds, so that the blaster can output the modem status. See [libftdi ftdi.c](https://github.com/lipro/libftdi/blob/master/src/ftdi.c#L2053) for more on this.
    /// Otherwise, [a BSOD could occur on Windows](https://github.com/mithro/ixo-usb-jtag/blob/master/usbjtag.c#L212)
    /// A safe default for the heartbeat seems to be true all the time. This will output the modem status whenever the host reads the device.
    pub fn write(&mut self, heartbeat: bool) -> usb_device::Result<usize> {
        if self.send_len == 0 && !heartbeat {
            return Err(UsbError::WouldBlock);
        }
        self.send_buffer[0] = FTDI_MODEM_STA_DUMMY[0];
        self.send_buffer[1] = FTDI_MODEM_STA_DUMMY[1];
        let res = self.class.write(&self.send_buffer[..self.send_len + 2]);
        if res.is_ok() {
            let amount = *res.as_ref().unwrap();
            if amount <= 2 {
                if amount == 1 {
                    // TODO: how to handle a half-sent STA?
                    panic!("Cannot recover from half-sent status");
                }
            } else {
                let actual_amount = amount - 2;
                for i in 0..(self.send_len - actual_amount) {
                    self.send_buffer[i + 2] = self.send_buffer[i + 2 + actual_amount];
                }
                self.send_len -= actual_amount;
            }
        }
        res
    }

    /// Runs all pending operations from the internal read buffer until either no operations are left or the internal write buffer is full.
    /// If a GPIO error occurs, the JTAG state machine will enter an undefined state requiring a forced USB bus reset.
    pub fn handle(&mut self) -> Result<(), E> {
        self.port.handle(
            &mut self.recv_buffer,
            &mut self.recv_len,
            &mut self.send_buffer[2..],
            &mut self.send_len,
        )
    }
}

impl<
        B,
        E,
        TDI: OutputPin<Error = E>,
        TCK: OutputPin<Error = E>,
        TMS: OutputPin<Error = E>,
        TDO: InputPin<Error = E>,
    > UsbClass<B> for Blaster<'_, B, E, TDI, TCK, TMS, TDO>
where
    B: UsbBus,
    E: core::fmt::Debug,
{
    fn get_configuration_descriptors(
        &self,
        writer: &mut DescriptorWriter,
    ) -> usb_device::Result<()> {
        self.class.get_configuration_descriptors(writer)
    }

    fn reset(&mut self) {
        self.class.reset();
        // TODO: if this fails, there are bigger, device-level problems.
        self.port.reset().expect("unable to reset port");
        self.send_len = 0;
        self.recv_len = 0;
    }

    fn control_in(&mut self, xfer: ControlIn<B>) {
        self.class.control_in(xfer);
    }

    fn control_out(&mut self, xfer: ControlOut<B>) {
        let req = xfer.request();

        if req.request_type == RequestType::Vendor {
            /// See [Linux kernel ftdi_sio.h](https://github.com/torvalds/linux/blob/master/drivers/usb/serial/ftdi_sio.h#L74)
            const FTDI_VEN_REQ_RESET: u8 = 0x00;
            /// [Set chip baud rate](https://github.com/torvalds/linux/blob/master/drivers/usb/serial/ftdi_sio.h#L104)
            const _FTDI_VEN_REQ_SET_BAUDRATE: u8 = 0x01;
            /// [Set RS232 line characteristics](https://github.com/torvalds/linux/blob/master/drivers/usb/serial/ftdi_sio.h#L198)
            const _FTDI_VEN_REQ_SET_DATA_CHAR: u8 = 0x02;
            /// [Set chip flow control](https://github.com/torvalds/linux/blob/master/drivers/usb/serial/ftdi_sio.h#L277)
            const _FTDI_VEN_REQ_SET_FLOW_CTRL: u8 = 0x03;
            /// [Set modem ctrl](https://github.com/torvalds/linux/blob/master/drivers/usb/serial/ftdi_sio.h#L232)
            const _FTDI_VEN_REQ_SET_MODEM_CTRL: u8 = 0x04;
            /// [Set special event character](https://github.com/torvalds/linux/blob/master/drivers/usb/serial/ftdi_sio.h#L365)
            const _FTDI_VEN_REQ_SET_EVENT_CHAR: u8 = 0x06;
            /// [Set parity error replacement character](https://github.com/torvalds/linux/blob/master/drivers/usb/serial/ftdi_sio.h#L382)
            const _FTDI_VEN_REQ_SET_ERR_CHAR: u8 = 0x07;
            /// [Set latency timer](https://github.com/torvalds/linux/blob/master/drivers/usb/serial/ftdi_sio.h#L324)
            const _FTDI_VEN_REQ_SET_LAT_TIMER: u8 = 0x09;
            /// [Set bitmode](https://github.com/lipro/libftdi/blob/master/src/ftdi.c#L1921)
            const _FTDI_VEN_REQ_SET_BITMODE: u8 = 0x0B;
            /// See [libftdi ftdi.h](https://github.com/lipro/libftdi/blob/master/src/ftdi.h#L169)
            /// This request is rejected -- EEPROM is read-only.
            const FTDI_VEN_REQ_WR_EEPROM: u8 = 0x91;
            /// This request is rejected -- EEPROM is read-only.
            const FTDI_VEN_REQ_ES_EEPROM: u8 = 0x92;
            match req.request {
                FTDI_VEN_REQ_RESET => {
                    const RESET_SIO: u16 = 0x0000;
                    const RESET_PURGE_RX: u16 = 0x0001;
                    const RESET_PURGE_TX: u16 = 0x0002;
                    match req.value {
                        RESET_SIO => {
                            self.reset();
                            xfer.accept().unwrap();
                        }
                        RESET_PURGE_RX => {
                            self.recv_len = 0;
                            xfer.accept().unwrap();
                        }
                        RESET_PURGE_TX => {
                            self.send_len = 0;
                            xfer.accept().unwrap();
                        }
                        _ => {
                            xfer.reject().unwrap();
                        }
                    }
                }
                FTDI_VEN_REQ_WR_EEPROM => {
                    xfer.reject().unwrap();
                }
                FTDI_VEN_REQ_ES_EEPROM => {
                    xfer.reject().unwrap();
                }
                _ => {
                    xfer.accept().unwrap();
                }
            }
        }
    }
}
