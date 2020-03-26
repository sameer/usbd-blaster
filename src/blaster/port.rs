use core::ops::Index;
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
    Undefined,
}

impl JTAGState {
    fn advance(&self, mode: bool) -> JTAGState {
        use JTAGState::*;
        match (self, mode) {
            (Reset, false) => RunIdle,
            (RunIdle, true) => SelectDR,
            (SelectIR, false) => CaptureIR,
            (SelectIR, true) => Reset,
            (CaptureIR, false) => ShiftIR,
            (CaptureIR, true) => Exit1IR,
            (ShiftIR, true) => Exit1IR,
            (Exit1IR, false) => PauseIR,
            (Exit1IR, true) => UpdateIR,
            (PauseIR, true) => Exit2IR,
            (Exit2IR, false) => ShiftIR,
            (Exit2IR, true) => UpdateIR,
            (UpdateIR, false) => RunIdle,
            (UpdateIR, true) => SelectDR,
            (SelectDR, false) => CaptureDR,
            (SelectDR, true) => SelectIR,
            (CaptureDR, false) => ShiftDR,
            (CaptureDR, true) => Exit1DR,
            (ShiftDR, true) => Exit1DR,
            (Exit1DR, false) => PauseDR,
            (Exit1DR, true) => UpdateDR,
            (PauseDR, true) => Exit2DR,
            (Exit2DR, false) => ShiftDR,
            (Exit2DR, true) => UpdateDR,
            (UpdateDR, false) => RunIdle,
            (UpdateDR, true) => SelectDR,
            _ => self.clone(),
        }
    }
}

impl Into<u8> for JTAGState {
    fn into(self) -> u8 {
        use JTAGState::*;
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
            Undefined => 16,
        }
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

    pub fn handle<
        A1: arrayvec::Array<Item = u8, Index = u16>,
        A2: arrayvec::Array<Item = u8, Index = u16>,
    >(
        &mut self,
        receive: &mut arrayvec::ArrayVec<A1>,
        send: &mut arrayvec::ArrayVec<A2>,
    ) {
        let mut i = 0;
        while i < receive.len() {
            if send.len() == send.capacity() - 2 {
                break;
            }

            let d = receive[i];
            if self.shift_count == 0 {
                // bit-bang mode (default)
                let shift_en = (d & Self::BLASTER_STA_SHIFT) != 0;
                self.read_en = (d & Self::BLASTER_STA_READ) != 0;
                if shift_en {
                    self.shift_count = d & Self::BLASTER_STA_CNT_MASK;
                } else {
                    self.set_state(d);
                    if self.read_en {
                        send.push(self.get_state());
                    }
                }
            } else {
                // shift-mode
                if self.read_en {
                    send.push(self.shift_io(d));
                } else {
                    self.shift_out(d);
                }
                self.shift_count -= 1;
            }
            i += 1;
        }
        for _i in 0..i {
            receive.pop_at(0);
        }
    }

    fn advance(&mut self, mode: bool, drive_signal: bool) {
        if drive_signal {
            if mode {
                self.tms.set_high().unwrap();
            } else {
                self.tms.set_low().unwrap();
            }
            self.tck.set_high().unwrap();
            self.tck.set_low().unwrap();
        }
        self.jtag_state = self.jtag_state.advance(mode);
    }

    pub fn set_state(&mut self, state: u8) {
        if (state & Self::BLASTER_STA_OUT_TDI) >> 4 != 0 {
            self.tdi.set_high().unwrap();
        } else {
            self.tdi.set_low().unwrap();
        }
        let tms_state = state & Self::BLASTER_STA_OUT_TMS != 0;
        if tms_state {
            self.tms.set_high().unwrap();
        } else {
            self.tms.set_low().unwrap();
        }
        let clk = state & Self::BLASTER_STA_OUT_TCK != 0;
        if self.got_clock && !clk {
            self.advance(tms_state, false);
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
        d |= 1 << Self::BLASTER_STA_IN_DATAOUT_BIT;
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
                shift_data |= 1u8 << 7;
            }
            self.tck.set_low().unwrap();
        }
        shift_data
    }
}
