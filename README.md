# USB Blaster for Arduino MKR Vidor 4000

## What is this?

A project for the [Arduino MKR Vidor 4000](https://www.arduino.cc/en/Guide/MKRVidor4000) written in [Rust](https://www.rust-lang.org/) that lets you program the onboard [FPGA](https://en.wikipedia.org/wiki/Field-programmable_gate_array) with [Quartus](https://en.wikipedia.org/wiki/Intel_Quartus_Prime).

## Usage

### Requirements

* Rust language
* arm-none-eabi-gcc (ArchLinux users, get gcc-arm-none-eabi-bin)
* [Atmel SAM flashing tool](https://github.com/shumatech/BOSSA) (aka bossac, comes in Arduino tools)

### Flashing the USB Blaster

```bash
cargo build --release
arm-none-eabi-objcopy -O binary target/thumbv6m-none-eabi/release/usbblaster-rs target/usbblaster-rs.bin
# Manual step: push reset button twice in quick succession to enter flash mode
bossac -i -d -U true -i -e -w -v target/usbblaster-rs.bin -R
```

### Using the USB Blaster

#### Intel (Altera) Quartus

```bash
# Verify that the blaster exists
jtagconfig
# Flash your FPGA
quartus_pgm -m jtag -o 'p;project-name.sof'
```

#### OpenOCD

```bash
openocd -f /usr/share/openocd/scripts/interface/altera-usb-blaster.cfg
```

You can safely ignore the following error:

`Error: IR capture error at bit 2, saw 0x3FFFFFFFFFFFFD55 not 0x...3`


## How it works

### USB

The board is set up as a USB device with the same VendorId and ProductId as an Altera USB Blaster.

The blaster communicates via a vendor-specific interface (Class = 255, SubClass = 255, Protocol = 255). When vendor-typed control requests are received, it emulates the ROM and the responses of the [FTDI245 chip](https://www.ftdichip.com/Products/ICs/FT245R.htm).

Just like the FT245, endpoint 1 is input-only and endpoint 2 is output-only. These are used to control blaster operation.

### Blaster

The blaster has two operating modes: bit-bang (default) or shift. In bit-bang, there is direct control of the JTAG lines; every received byte translates to instructions on how to drive TDI/TMS/TCK. It also contains flags for whether this instruction is a read or write, and if the blaster should switch to shift mode and shift out the next n bytes. In shift mode, the blaster will shift out the next n (anywhere from 0 to 63) received bytes to the TDI line.

Bit-bang mode is useful for JTAG control, shift mode is useful for a bulk transfer like writing an FPGA bitstream.

## To-Do

- [ ] Make pull requests for changes made
    - [x] LCK_C & LCK_F
    - [ ] SRAM QoS for Arduino USB
    - [ ] Enable FPGA Clock equivalent
- [ ] Document everything in this repo

## Special Thanks

* [Martino Facchin](https://github.com/facchinm)

## Reference Documents

* [SAMD21 Family Data Sheet](http://ww1.microchip.com/downloads/en/DeviceDoc/SAM_D21_DA1_Family_DataSheet_DS40001882F.pdf)


## Rust atsamd21g18a hal comparison with Arduino SAMD core investigation

- Confirm entire USB implementation matches Arduino SAMD Core
    - [x] UsbBus::new matches UsbDeviceClas::init
        - Dp/Dm pin setup correct
    - [x] Clock enable correct
        - GCLK 0 set as source for GCLK6 with just clock enable, not clock generate
        - waits for sync
    - [x] Clock generation correct
        - Sets half enable for NVM
        - Sets apba mask
        - Use external oscillator (!! switched to this)
        - Correctly writes genclk 1 configuration (no idc)
        - Feeds 32k to DFLL48
        - Everything else looked correct: just there was a missing sync or two & there was no wait for LCK_C, LCK_F
    - [ ] USB Init
        - Correct multiplexer (6)
        - Correct reset: swrst on ctrla
        - Calibration
            - Correct NVM calibration data & addresses on UsbBus::enable
        - Run in standby
        - Device mode
        - Full speed
        - NVIC interrupt with priority 0
        - [ ] there are some extra steps I noticed in UsbBus::enable
            - SRAM QoS (memory priority access) set to critical for data and configuration
            - clear pending on intflag (?)
            - flush endpoints (host behavior compensation)
    - [x] Attach/enable
        - [x] UsbBus::enable does not enable start of frame interrupt
            - Arduino uses SoF interrupt to flash LEDs on boards with TX/RX LEDs
        - Detach clear_bit correct
    - [ ] Polling
        - idek
    - [ ] PluggableUSB vs usb-device: blaster implementation
        - no handleEndpoint -- just does everything in the main program loop it seems, without interrupts
    - [ ] Implementation specific values
        - Arduino has pack messages to aggregate sending (unused by blaster I guess)