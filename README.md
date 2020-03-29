# USB Blaster for Arduino MKR Vidor 4000

## Debug To-do

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
    - [ ] Attach/enable
        - [ ] UsbBus::enable does not enable start of frame interrupt
        - Detach clear_bit correct
