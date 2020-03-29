use hal::usb::usb_device::{class_prelude::*, control::RequestType, prelude::*, Result};

mod class;
mod ft245;
mod port;

use class::BlasterClass;

pub const ALTERA_BLASTER_USB_VID_PID: UsbVidPid = UsbVidPid(0x09FB, 0x6001);

// Depending on the underlying USB library (libusb or similar) the OS may send/receive more bytes than declared in the USB endpoint
// This will change the endpoint size (OS side) so it's less likely to send more than 64 bytes in a single chunk.
const BLASTER_WRITE_SIZE: usize = 64;
const BLASTER_READ_SIZE: usize = 64;

pub struct USBBlaster<'a, B: UsbBus> {
    class: BlasterClass<'a, B>,
    port: port::Port,
    send_buffer: [u8; BLASTER_WRITE_SIZE],
    send_len: usize,
    recv_buffer: [u8; BLASTER_READ_SIZE],
    recv_len: usize,
    first_send: bool,
}

impl<'a, B: UsbBus> USBBlaster<'a, B> {
    pub fn new(
        alloc: &'a UsbBusAllocator<B>,
        tdi: hal::gpio::Pa12<hal::gpio::Output<hal::gpio::PushPull>>,
        tck: hal::gpio::Pa13<hal::gpio::Output<hal::gpio::PushPull>>,
        tms: hal::gpio::Pa14<hal::gpio::Output<hal::gpio::PushPull>>,
        tdo: hal::gpio::Pa15<hal::gpio::Input<hal::gpio::Floating>>,
    ) -> USBBlaster<'a, B> {
        USBBlaster {
            class: BlasterClass::new(alloc, BLASTER_WRITE_SIZE as u16, BLASTER_READ_SIZE as u16),
            port: port::Port::new(tdi, tck, tms, tdo),
            send_buffer: [0u8; BLASTER_WRITE_SIZE],
            send_len: 0,
            recv_buffer: [0u8; BLASTER_READ_SIZE],
            recv_len: 0,
            first_send: true,
        }
    }

    pub fn read(&mut self) -> Result<usize> {
        if self.recv_len == self.recv_buffer.len() {
            return Err(UsbError::WouldBlock)
        }
        let amount = self.class.read(&mut self.recv_buffer[self.recv_len..])?;
        self.recv_len += amount;
        Ok(amount)
    }

    pub fn write(&mut self, heartbeat: bool) -> Result<usize> {
        if !(self.send_len != 0 || self.first_send || heartbeat) {
            return Err(UsbError::WouldBlock);
        }
        self.send_buffer[0] = BlasterClass::<'_, B>::FTDI_MODEM_STA_DUMMY[0];
        self.send_buffer[1] = BlasterClass::<'_, B>::FTDI_MODEM_STA_DUMMY[1];
        self.first_send = false;
        let res = self
            .class
            .write(&self.send_buffer[..self.send_len + 2]);
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

    pub fn handle(&mut self) {
        self.port
            .handle(&mut self.recv_buffer, &mut self.recv_len, &mut self.send_buffer[2..], &mut self.send_len);
    }
}

impl<B> UsbClass<B> for USBBlaster<'_, B>
where
    B: UsbBus,
{
    fn get_configuration_descriptors(&self, writer: &mut DescriptorWriter) -> Result<()> {
        self.class.get_configuration_descriptors(writer)
    }

    fn reset(&mut self) {
        self.class.reset();
        self.port.reset();
        self.first_send = true;
        self.send_len = 0;
        self.recv_len = 0;
    }

    fn control_in(&mut self, xfer: ControlIn<B>) {
        self.class.control_in(xfer);
    }

    fn control_out(&mut self, xfer: ControlOut<B>) {
        let req = xfer.request();
        if req.request_type == RequestType::Vendor {
            // sendZLP equivalent
            match req.request {
                BlasterClass::<'_, B>::FTDI_VEN_REQ_RESET => {
                    match req.value {
                        0 => self.reset(),
                        1 => {
                            // self.read_ep.clear()
                            self.reset();
                        }
                        2 => {
                            // self.write_ep.clear()
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
