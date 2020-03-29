use hal::digital::v2::{InputPin, OutputPin};
use usb_device::{class_prelude::*, control::RequestType};

use crate::class::BlasterClass;
use crate::port::Port;

/// Depending on the underlying USB library (libusb or similar) the OS may send/receive more bytes than declared in the USB endpoint
/// If this happens to you, please open an issue for this crate on GitHub.
const BLASTER_WRITE_SIZE: usize = 64;
const BLASTER_READ_SIZE: usize = 64;

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
    first_send: bool,
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
    /// Allocate a Blaster on the USB bus. Gives control of the four JTAG pins.
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
            first_send: true,
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
    /// The heartbeat parameter must be true at least once every 10 milliseconds, so that the blaster can output the modem status.
    /// A safe default for the heartbeat seems to be true all the time. This will output the modem status whenever the host reads the device.
    pub fn write(&mut self, heartbeat: bool) -> usb_device::Result<usize> {
        if !(self.send_len != 0 || self.first_send || heartbeat) {
            return Err(UsbError::WouldBlock);
        }
        self.send_buffer[0] = BlasterClass::<'_, B>::FTDI_MODEM_STA_DUMMY[0];
        self.send_buffer[1] = BlasterClass::<'_, B>::FTDI_MODEM_STA_DUMMY[1];
        self.first_send = false;
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
    B: UsbBus, E: core::fmt::Debug
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
        self.first_send = true;
        self.send_len = 0;
        self.recv_len = 0;
    }

    fn control_in(&mut self, xfer: ControlIn<B>) {
        self.class.control_in(xfer);
    }

    fn control_out(&mut self, xfer: ControlOut<B>) {
        let req = xfer.request();
        if !(req.recipient == control::Recipient::Endpoint
            && req.index == BlasterClass::<'_, B>::INTERFACE_A_INDEX)
        {
            return;
        }

        if req.request_type == RequestType::Vendor {
            match req.request {
                BlasterClass::<'_, B>::FTDI_VEN_REQ_RESET => {
                    match req.value {
                        0 => self.reset(),
                        1 => {
                            // TODO: self.read_ep.clear()
                            self.reset();
                        }
                        2 => {
                            // TODO: self.write_ep.clear()
                            self.reset();
                        }
                        _ => {}
                    }
                    xfer.accept().unwrap();
                }
                BlasterClass::<'_, B>::FTDI_VEN_REQ_WR_EEPROM => {
                    xfer.reject().unwrap();
                }
                BlasterClass::<'_, B>::FTDI_VEN_REQ_ES_EEPROM => {
                    xfer.reject().unwrap();
                }
                _ => {
                    xfer.accept().unwrap();
                }
            }
        }
    }
}
