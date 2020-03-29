use hal::prelude::*;

pub struct Port {
    tdi: hal::gpio::Pa12<hal::gpio::Output<hal::gpio::PushPull>>,
    tck: hal::gpio::Pa13<hal::gpio::Output<hal::gpio::PushPull>>,
    tms: hal::gpio::Pa14<hal::gpio::Output<hal::gpio::PushPull>>,
    tdo: hal::gpio::Pa15<hal::gpio::Input<hal::gpio::Floating>>,
    jtag_state: JTAGState,
    shift_count: u8,
    read_en: bool,
    got_clock: bool,
}

#[derive(PartialEq, Clone)]
#[repr(u8)]
enum JTAGState {
    Reset,
    RunIdle,
    SelectIR,
    CaptureIR,
    ShiftIR,
    Exit1IR,
    PauseIR,
    Exit2IR,
    UpdateIR,
    SelectDR,
    CaptureDR,
    ShiftDR,
    Exit1DR,
    PauseDR,
    Exit2DR,
    UpdateDR,
    // Undefined,
}
use JTAGState::*;
impl JTAGState {
    const STATE_MACHINE: [[Self; 2]; 16] = [
        /*-State-      -mode= '0'-    -mode= '1'- */
        /*RESET     */ [RunIdle, Reset],
        /*RUNIDLE   */ [RunIdle, SelectDR],
        /*SELECTIR  */ [CaptureIR, Reset],
        /*CAPTURE_IR*/ [ShiftIR, Exit1IR],
        /*SHIFT_IR  */ [ShiftIR, Exit1IR],
        /*EXIT1_IR  */ [PauseIR, UpdateIR],
        /*PAUSE_IR  */ [PauseIR, Exit2IR],
        /*EXIT2_IR  */ [ShiftIR, UpdateIR],
        /*UPDATE_IR */ [RunIdle, SelectDR],
        /*SELECT_DR */ [CaptureDR, SelectIR],
        /*CAPTURE_DR*/ [ShiftDR, Exit1DR],
        /*SHIFT_DR  */ [ShiftDR, Exit1DR],
        /*EXIT1_DR  */ [PauseDR, UpdateDR],
        /*PAUSE_DR  */ [PauseDR, Exit2DR],
        /*EXIT2_DR  */ [ShiftDR, UpdateDR],
        /*UPDATE_DR */ [RunIdle, SelectDR],
    ];
    fn advance(&self, mode: bool) -> Self {
        let idx: u8 = self.clone().into();
        let mode = if mode { 1 } else { 0 };
        Self::STATE_MACHINE[idx as usize][mode].clone()
    }
}

impl Into<u8> for JTAGState {
    fn into(self) -> u8 {
        match self {
            Reset => 0,
            RunIdle => 1,
            SelectIR => 2,
            CaptureIR => 3,
            ShiftIR => 4,
            Exit1IR => 5,
            PauseIR => 6,
            Exit2IR => 7,
            UpdateIR => 8,
            SelectDR => 9,
            CaptureDR => 10,
            ShiftDR => 11,
            Exit1DR => 12,
            PauseDR => 13,
            Exit2DR => 14,
            UpdateDR => 15,
            // Undefined => 16,
        }
    }
}

impl Default for JTAGState {
    fn default() -> Self {
        Self::Reset
    }
}

impl Port {
    // mode set
    const BLASTER_STA_SHIFT: u8 = 0x80;
    const BLASTER_STA_READ: u8 = 0x40;
    const BLASTER_STA_CNT_MASK: u8 = 0x3f;

    // bit-bang out
    const BLASTER_STA_OUT_OE: u8 = 0x20;
    const BLASTER_STA_OUT_TDI: u8 = 0x10;
    const BLASTER_STA_OUT_NCS: u8 = 0x08;
    const BLASTER_STA_OUT_NCE: u8 = 0x04;
    const BLASTER_STA_OUT_TMS: u8 = 0x02;
    const BLASTER_STA_OUT_TCK: u8 = 0x01;

    // bit-bang in
    const BLASTER_STA_IN_TDO: u8 = 0x01;
    const BLASTER_STA_IN_DATAOUT: u8 = 0x02;

    const BLASTER_STA_IN_TDO_BIT: u8 = 0;
    const BLASTER_STA_IN_DATAOUT_BIT: u8 = 1;

    pub fn new(
        tdi: hal::gpio::Pa12<hal::gpio::Output<hal::gpio::PushPull>>,
        tck: hal::gpio::Pa13<hal::gpio::Output<hal::gpio::PushPull>>,
        tms: hal::gpio::Pa14<hal::gpio::Output<hal::gpio::PushPull>>,
        tdo: hal::gpio::Pa15<hal::gpio::Input<hal::gpio::Floating>>,
    ) -> Port {
        Port {
            tdi,
            tck,
            tms,
            tdo,
            jtag_state: JTAGState::Reset,
            shift_count: 0,
            read_en: false,
            got_clock: false,
        }
    }

    #[inline]
    pub fn handle(
        &mut self,
        recv_buf: &mut [u8],
        recv_len: &mut usize,
        send_buf: &mut [u8],
        send_len: &mut usize,
    ) {
        let mut i = 0usize;
        while i < *recv_len && *send_len < send_buf.len() {
            let d = recv_buf[i];
            if self.shift_count == 0 {
                // bit-bang mode (default)
                self.read_en = (d & Self::BLASTER_STA_READ) != 0;
                if d & Self::BLASTER_STA_SHIFT != 0 { // Swap to shift mode for 0 to 63 shifts
                    self.shift_count = d & Self::BLASTER_STA_CNT_MASK;
                } else {
                    self.set_state(d);
                    if self.read_en {
                        send_buf[*send_len] = self.get_state();
                        *send_len += 1;
                    }
                }
            } else {
                // shift-mode
                if self.read_en {
                    send_buf[*send_len] = self.shift_io(d);
                    *send_len += 1;
                } else {
                    self.shift_out(d);
                }
                self.shift_count -= 1;
            }
            i += 1;
        }
        if i != 0 {
            for j in 0..(*recv_len - i) {
                recv_buf[j] = recv_buf[j + i];
            }
            *recv_len -= i;
        }
    }

    fn advance(&mut self, mode: bool) {
        self.jtag_state = self.jtag_state.advance(mode);
    }

    pub fn set_state(&mut self, d: u8) {
        if (d & Self::BLASTER_STA_OUT_TDI) >> 4 != 0 {
            self.tdi.set_high().unwrap();
        } else {
            self.tdi.set_low().unwrap();
        }
        let tms = ((d & Self::BLASTER_STA_OUT_TMS) >> 1) != 0;
        if tms {
            self.tms.set_high().unwrap();
        } else {
            self.tms.set_low().unwrap();
        }
        let clk = d & Self::BLASTER_STA_OUT_TCK != 0;
        if self.got_clock && !clk {
            self.advance(tms);
            self.got_clock = false;
        }
        if clk {
            self.got_clock = true;
            self.tck.set_high().unwrap();
        } else {
            self.tck.set_low().unwrap();
        }
    }

    pub fn get_state(&mut self) -> u8 {
        let mut d = 0u8;
        if self.tdo.is_high().unwrap() {
            d |= 1 << Self::BLASTER_STA_IN_TDO_BIT;
        }
        // d |= 1 << Self::BLASTER_STA_IN_DATAOUT_BIT;
        d
    }

    pub fn reset(&mut self) {
        self.jtag_state = JTAGState::Reset;
        self.shift_count = 0;
        self.read_en = false;
        self.got_clock = false;
    }

    fn shift_out(&mut self, data: u8) {
        let mut shift_data = data;
        for _i in 0..8 {
            if shift_data & 1 != 0 {
                self.tdi.set_high().unwrap();
            } else {
                self.tdi.set_low().unwrap();
            }
            self.tck.set_high().unwrap();
            shift_data >>= 1;
            self.tck.set_low().unwrap();
        }
    }

    fn shift_io(&mut self, data: u8) -> u8 {
        let mut shift_data = data;
        for _i in 0..8 {
            if shift_data & 1 != 0 {
                self.tdi.set_high().unwrap();
            } else {
                self.tdi.set_low().unwrap();
            }
            let din = self.tdo.is_high().unwrap();
            self.tck.set_high().unwrap();
            shift_data >>= 1;
            if din {
                shift_data |= 0b1000_0000u8;
            }
            self.tck.set_low().unwrap();
        }
        shift_data
    }
}
