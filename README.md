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
        - Use external oscillator (!!)
        - Correctly writes genclk 1 configuration (no idc)
        - Feeds 32k to DFLL48
        - Everything else looked correct: just there was a missing sync or two & there was no wait for LCK_C, LCK_F
    