use usb_device::{class_prelude::*, control::RequestType, Result, UsbDirection};

use super::ft245::ROM;

/// See [ftdi.h](https://github.com/lipro/libftdi/blob/master/src/ftdi.c#L2049)
const DATA_READY: u8 = 0b0000_0001;
const RECEIVE_LINE_SIGNAL_DETECT_ACTIVE: u8 = 0b1000_0000;
const RING_INDICATOR_ACTIVE: u8 = 0b0100_0000;
pub const FTDI_MODEM_STA_DUMMY: [u8; 2] = [DATA_READY, RECEIVE_LINE_SIGNAL_DETECT_ACTIVE | RING_INDICATOR_ACTIVE];

pub struct BlasterClass<'a, B: UsbBus> {
    iface: InterfaceNumber,
    pub read_ep: EndpointOut<'a, B>,
    pub write_ep: EndpointIn<'a, B>,
    _fake_write_ep: EndpointIn<'a, B>,
    _fake_read_ep: EndpointOut<'a, B>,
}

impl<'a, B: UsbBus> UsbClass<B> for BlasterClass<'a, B> {
    fn get_configuration_descriptors(&self, w: &mut DescriptorWriter) -> Result<()> {
        w.interface(self.iface, 0xFF, 0xFF, 0xFF)?;
        w.endpoint(&self.write_ep)?;
        w.endpoint(&self.read_ep)
    }

    fn reset(&mut self) {}

    fn control_in(&mut self, xfer: ControlIn<B>) {
        /// [Get modem status](https://github.com/lipro/libftdi/blob/master/src/ftdi.c#L2049)
        const FTDI_VEN_REQ_GET_MODEM_STA: u8 = 0x05;
        /// [Get latency timer](https://github.com/torvalds/linux/blob/master/drivers/usb/serial/ftdi_sio.h#L302)
        const FTDI_VEN_REQ_GET_LAT_TIMER: u8 = 0x0A;
        /// [Read pins](https://github.com/lipro/libftdi/blob/master/src/ftdi.c#L1972)
        const _FTDI_VEN_REQ_RD_PINS: u8 = 0x0C;
        //// [Read EEPROM location](https://github.com/lipro/libftdi/blob/master/src/ftdi.c#L4025)
        const FTDI_VEN_REQ_RD_EEPROM: u8 = 0x90;

        /// Must be a value between 1 and 255
        /// [16 is the default](https://github.com/torvalds/linux/blob/master/drivers/usb/serial/ftdi_sio.h#L310)
        const FTDI_LAT_TIMER_DUMMY: [u8; 1] = ['6' as u8];

        let req = xfer.request();
        if req.request_type == RequestType::Vendor {
            match req.request {
                FTDI_VEN_REQ_RD_EEPROM => {
                    let addr = (((req.value >> 8) & 0x3f) << 1) as usize;
                    xfer.accept_with(&ROM[addr..=addr + 1]).unwrap();
                }
                FTDI_VEN_REQ_GET_MODEM_STA => {
                    xfer.accept_with_static(&FTDI_MODEM_STA_DUMMY)
                        .unwrap();
                }
                FTDI_VEN_REQ_GET_LAT_TIMER => {
                    xfer.accept_with_static(&FTDI_LAT_TIMER_DUMMY)
                        .unwrap();
                }
                _ => {
                    xfer.accept_with_static(&[0u8; 2]).unwrap();
                }
            }
        }
    }
}

impl<B: UsbBus> BlasterClass<'_, B> {
    pub fn new(
        alloc: &UsbBusAllocator<B>,
        max_write_packet_size: u16,
        max_read_packet_size: u16,
    ) -> BlasterClass<'_, B> {
        BlasterClass {
            iface: alloc.interface(),
            /// See INTERFACE_A: https://github.com/lipro/libftdi/blob/master/src/ftdi.c#L178
            _fake_read_ep: alloc
                .alloc(
                    Some(EndpointAddress::from_parts(0x01, UsbDirection::Out)),
                    EndpointType::Bulk,
                    max_write_packet_size,
                    1,
                )
                .expect("alloc_ep failed"),
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
            _fake_write_ep: alloc
                .alloc(
                    Some(EndpointAddress::from_parts(0x02, UsbDirection::In)),
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
