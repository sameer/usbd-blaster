use usb_device::{class_prelude::*, control::RequestType, Result, UsbDirection};

use super::ft245::ROM;

pub struct BlasterClass<'a, B: UsbBus> {
    iface: InterfaceNumber,
    pub read_ep: EndpointOut<'a, B>,
    pub write_ep: EndpointIn<'a, B>,
}

impl<'a, B: UsbBus> UsbClass<B> for BlasterClass<'a, B> {
    fn get_configuration_descriptors(&self, w: &mut DescriptorWriter) -> Result<()> {
        w.interface(self.iface, 0xFF, 0xFF, 0xFF)?;
        w.endpoint(&self.write_ep)?;
        w.endpoint(&self.read_ep)
    }

    fn reset(&mut self) {}

    fn control_in(&mut self, xfer: ControlIn<B>) {
        let req = xfer.request();
        if !(req.recipient == control::Recipient::Endpoint && req.index == Self::INTERFACE_A_INDEX)
        {
            return;
        }
        if req.request_type == RequestType::Vendor {
            match req.request {
                Self::FTDI_VEN_REQ_RD_EEPROM => {
                    let addr = (((req.value >> 8) & 0x3f) << 1) as usize;
                    xfer.accept_with(&[ROM[addr], ROM[addr + 1]]).unwrap();
                }
                Self::FTDI_VEN_REQ_GET_MODEM_STA => {
                    xfer.accept_with_static(&Self::FTDI_MODEM_STA_DUMMY)
                        .unwrap();
                }
                Self::FTDI_VEN_REQ_GET_LAT_TIMER => {
                    xfer.accept_with_static(&Self::FTDI_LAT_TIMER_DUMMY)
                        .unwrap();
                }
                _ => {
                    xfer.accept_with_static(&[0u8; 2]).unwrap();
                }
            }
        } else {
            xfer.reject().ok();
        }
    }
}

impl<B: UsbBus> BlasterClass<'_, B> {
    pub const FTDI_VEN_REQ_RESET: u8 = 0x00;
    const FTDI_VEN_REQ_SET_BAUDRATE: u8 = 0x01;
    const FTDI_VEN_REQ_SET_DATA_CHAR: u8 = 0x02;
    const FTDI_VEN_REQ_SET_FLOW_CTRL: u8 = 0x03;
    const FTDI_VEN_REQ_SET_MODEM_CTRL: u8 = 0x04;
    const FTDI_VEN_REQ_GET_MODEM_STA: u8 = 0x05;
    const FTDI_VEN_REQ_SET_EVENT_CHAR: u8 = 0x06;
    const FTDI_VEN_REQ_SET_ERR_CHAR: u8 = 0x07;
    const FTDI_VEN_REQ_SET_LAT_TIMER: u8 = 0x09;
    const FTDI_VEN_REQ_GET_LAT_TIMER: u8 = 0x0A;
    const FTDI_VEN_REQ_SET_BITMODE: u8 = 0x0B;
    const FTDI_VEN_REQ_RD_PINS: u8 = 0x0C;
    const FTDI_VEN_REQ_RD_EEPROM: u8 = 0x90;
    pub const FTDI_VEN_REQ_WR_EEPROM: u8 = 0x91;
    pub const FTDI_VEN_REQ_ES_EEPROM: u8 = 0x92;

    pub const FTDI_MODEM_STA_DUMMY: [u8; 2] = [0x01, 0x60];

    /// Must be a value between 1 and 255
    const FTDI_LAT_TIMER_DUMMY: [u8; 1] = ['6' as u8];

    pub const INTERFACE_A_INDEX: u16 = 1;

    pub fn new(
        alloc: &UsbBusAllocator<B>,
        max_write_packet_size: u16,
        max_read_packet_size: u16,
    ) -> BlasterClass<'_, B> {
        BlasterClass {
            iface: alloc.interface(),
            write_ep: alloc
                .alloc(
                    Some(EndpointAddress::from_parts(0x01, UsbDirection::In)),
                    EndpointType::Bulk,
                    max_write_packet_size,
                    1,
                )
                .expect("alloc_ep failed"),
            read_ep: alloc
                .alloc(
                    Some(EndpointAddress::from_parts(0x02, UsbDirection::Out)),
                    EndpointType::Bulk,
                    max_read_packet_size,
                    1,
                )
                .expect("alloc_ep failed"),
        }
    }

    pub fn read(&mut self, data: &mut [u8]) -> Result<usize> {
        self.read_ep.read(data)
    }

    pub fn write(&mut self, data: &[u8]) -> Result<usize> {
        self.write_ep.write(data)
    }
}