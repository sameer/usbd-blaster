use hal::usb::usb_device::descriptor::descriptor_type::*;
const FT245ROM_SIZE: usize = (128);
const FT245ROM_STR_LIMIT: usize = (100);

const BLASTER_SIZ_DEVICE_DESC: u8 = 18;
const BLASTER_SIZ_CONFIG_DESC: u8 = 32;
const BLASTER_SIZ_STRING_LANGID: u8 = 4;
const BLASTER_SIZ_STRING_VENDOR: u8 = 14;
const BLASTER_SIZ_STRING_PRODUCT: u8 = 24;
const BLASTER_SIZ_STRING_SERIAL: u8 = 18;

/* USB Standard Device Descriptor */
const Blaster_DeviceDescriptor: [u8; BLASTER_SIZ_DEVICE_DESC as usize] = [
    0x12,   /*bLength */
    DEVICE, /*bDescriptorType*/
    0x00,   /*bcdUSB */
    0x02, 0x00, /*bDeviceClass*/
    0x00, /*bDeviceSubClass*/
    0x00, /*bDeviceProtocol*/
    0x40, /*bMaxPacketSize(64bytes)*/
    0xFB, /*idVendor (0x09FB=Altera)*/
    0x09, 0x01, /*idProduct(0x6001=USB-Blaster)*/
    0x60, 0x00, /*bcdDevice rel. B*/
    0x04, 1,    /*Index of string descriptor describing manufacturer */
    2,    /*Index of string descriptor describing product*/
    3,    /*Index of string descriptor describing the device serial number */
    0x01, /*bNumConfigurations*/
];

/* USB Configuration Descriptor */
/*   All Descriptors (Configuration, Interface, Endpoint, Class, Vendor */
const Blaster_ConfigDescriptor: [u8; BLASTER_SIZ_CONFIG_DESC as usize] = [
    /* Configuration Descriptor */
    /* 00 */
    0x09,                    /* bLength: Configuration Descriptor size */
    CONFIGURATION,           /* bDescriptorType: Configuration */
    BLASTER_SIZ_CONFIG_DESC, /* wTotalLength: Bytes returned */
    0x00,
    0x01, /* bNumInterfaces: 1 interface */
    0x01, /* bConfigurationValue: Configuration value */
    0x00, /* iConfiguration: Index of string descriptor describing the configuration*/
    0x80, /* bmAttributes: Bus powered(bit6=0) */
    0xE1, /* MaxPower 450mA(225*2) */
    /* Interface Descriptor */
    /* 09 */
    0x09,      /* bLength: Interface Descriptor size */
    INTERFACE, /* bDescriptorType: Interface descriptor type */
    0x00,      /* bInterfaceNumber: Number of Interface */
    0x00,      /* bAlternateSetting: Alternate setting */
    2 - 1,     /* bNumEndpoints */
    0xFF,      /* bInterfaceClass: NA */
    0xFF,      /* bInterfaceSubClass : NA */
    0xFF,      /* nInterfaceProtocol : NA */
    0,         /* iInterface: Index of string descriptor */
    /* Endpoint Descriptor */
    /* 18 */
    0x07,     /* bLength: Endpoint Descriptor size */
    ENDPOINT, /* bDescriptorType: Endpoint descriptor */
    2,        /* bEndpointAddress: Endpoint 1 IN */
    0x02,     /* bmAttributes: Bulk endpoint */
    64,       /* wMaxPacketSize: 64 Bytes max */
    0x00,
    0x01, /* bInterval: Polling Interval (1 ms) */
    /* 25 */
    0x07,     /* bLength: Endpoint Descriptor size */
    ENDPOINT, /* bDescriptorType: Endpoint descriptor */
    3,        /* bEndpointAddress: Endpoint 2 OUT */
    0x02,     /* bmAttributes: Bulk endpoint */
    64,       /* wMaxPacketSize: 64 Bytes max  */
    0x00,
    0x01, /* bInterval: Polling Interval (1 ms) */
          /* 32 */
];

const Blaster_StringVendor: [u8; BLASTER_SIZ_STRING_VENDOR as usize] = [
    BLASTER_SIZ_STRING_VENDOR, /* Size of Vendor string */
    STRING,                    /* bDescriptorType*/
    'A' as u8,
    0,
    'l' as u8,
    0,
    't' as u8,
    0,
    'e' as u8,
    0,
    'r' as u8,
    0,
    'a' as u8,
    0, /* Manufacturer: "Altera" */
];

const Blaster_StringProduct: [u8; BLASTER_SIZ_STRING_PRODUCT as usize] = [
    BLASTER_SIZ_STRING_PRODUCT, /* bLength */
    STRING,                     /* bDescriptorType */
    'U' as u8,
    0,
    'S' as u8,
    0,
    'B' as u8,
    0,
    '-' as u8,
    0,
    'B' as u8,
    0,
    'l' as u8,
    0,
    'a' as u8,
    0,
    's' as u8,
    0,
    't' as u8,
    0,
    'e' as u8,
    0,
    'r' as u8,
    0, /* "USB-Blaster" */
];

const Blaster_StringSerial: [u8; BLASTER_SIZ_STRING_SERIAL as usize] = [
    BLASTER_SIZ_STRING_SERIAL, /* bLength */
    STRING,                    /* bDescriptorType */
    '1' as u8,
    0,
    '2' as u8,
    0,
    '3' as u8,
    0,
    '4' as u8,
    0,
    '5' as u8,
    0,
    '6' as u8,
    0,
    '7' as u8,
    0,
    '8' as u8,
    0, /* "12345678" */
];

pub struct Rom {
    pub buf: [u8; 128],
}
impl Rom {
    pub fn new() -> Rom {
        let mut pbuf = [0u8; 128];
        let mut offset = 0u8;

        // if (Blaster_StringVendor[0] + Blaster_StringProduct[0] + Blaster_StringSerial[0]
        //     > FT245ROM_STR_LIMIT)
        // {
        //     return -1;
        // }
        pbuf[0] = 0x0;
        pbuf[1] = 0x0;
        for i in 0..6 {
            pbuf[2 + i] = Blaster_DeviceDescriptor[8 + i]; // vid/pid/ver
        }
        pbuf[8] = Blaster_ConfigDescriptor[7]; // attr
        pbuf[9] = Blaster_ConfigDescriptor[8]; // pwr
        pbuf[10] = 0x1C; // chip config
        pbuf[11] = 0x00;
        pbuf[12] = Blaster_DeviceDescriptor[2]; // usb ver
        pbuf[13] = Blaster_DeviceDescriptor[3];
        // strings offset and length
        offset = 0x80 | (14 + 2 * 3);
        pbuf[14] = offset;
        pbuf[15] = Blaster_StringVendor[0];
        offset += Blaster_StringVendor[0];
        pbuf[16] = offset;
        pbuf[17] = Blaster_StringProduct[0];
        offset += Blaster_StringProduct[0];
        pbuf[18] = offset;
        pbuf[19] = Blaster_StringSerial[0];
        for i in 0..Blaster_StringVendor[0] as usize {
            pbuf[20 + i] = Blaster_StringVendor[i]; // vendor string
        }
        for i in 0..Blaster_StringProduct[0] as usize {
            pbuf[20 + i + Blaster_StringVendor[0] as usize] = Blaster_StringProduct[i];
            // product string
        }
        for i in 0..Blaster_StringSerial[0] as usize {
            pbuf[20 + i + Blaster_StringVendor[0] as usize + Blaster_StringProduct[0] as usize] =
                Blaster_StringSerial[i]; // serial string
        }

        let newidx = 20
            + Blaster_StringVendor[0] as usize
            + Blaster_StringProduct[0] as usize
            + Blaster_StringSerial[0] as usize;
        pbuf[newidx] = 0x2;
        pbuf[newidx + 1] = 0x3;
        pbuf[newidx + 2] = 0x1;
        pbuf[newidx + 3] = 0x0;
        pbuf[newidx + 4] = 'R' as u8 as u8;
        pbuf[newidx + 5] = 'E' as u8 as u8;
        pbuf[newidx + 6] = 'V' as u8 as u8;
        pbuf[newidx + 7] = 'B' as u8 as u8;
        // checksum
        let mut checksum = 0xAAAA;

        for i in 0..(128 - 2) / 2 {
            checksum ^= u16::from_le_bytes([pbuf[i], pbuf[i + 1]]);
            //   checksum ^= (pbuf[i + 1] << 8) as u16 | pbuf[i] as u16;
            checksum = (checksum << 1) | (checksum >> 15);
        }
        let checksum_as_bytes = checksum.to_le_bytes();
        pbuf[126] = checksum_as_bytes[0];
        pbuf[127] = checksum_as_bytes[1];
        Rom { buf: pbuf }
    }
}
