use hal::digital::v2::{InputPin, OutputPin};

pub struct Port<
    E,
    TDI: OutputPin<Error = E>,
    TCK: OutputPin<Error = E>,
    TMS: OutputPin<Error = E>,
    TDO: InputPin<Error = E>,
> {
    tdi: TDI,
    tck: TCK,
    tms: TMS,
    tdo: TDO,
    jtag_state: JTAGState,
    shift_count: u8,
    shift_data: u8,
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
use JTAGState::*;
impl JTAGState {
    const STATE_MACHINE: [[Self; 2]; 17] = [
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
        /*UNDEFINED */ [Undefined, Undefined],
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
            Undefined => 16,
        }
    }
}

impl Default for JTAGState {
    fn default() -> Self {
        Self::Reset
    }
}

impl<
        E,
        TDI: OutputPin<Error = E>,
        TCK: OutputPin<Error = E>,
        TMS: OutputPin<Error = E>,
        TDO: InputPin<Error = E>,
    > Port<E, TDI, TCK, TMS, TDO>
where
    TDI: OutputPin,
    TCK: OutputPin,
    TMS: OutputPin,
    TDO: InputPin,
{
    /// [Shift bit](https://github.com/mithro/ixo-usb-jtag/blob/master/usbjtag.c#L173)
    const BLASTER_STA_SHIFT: u8 = 0x80;
    /// [Read bit](https://github.com/mithro/ixo-usb-jtag/blob/master/usbjtag.c#L171)
    const BLASTER_STA_READ: u8 = 0x40;
    /// [Byte shift count mask](https://github.com/mithro/ixo-usb-jtag/blob/master/usbjtag.c#L174)
    const BLASTER_STA_CNT_MASK: u8 = 0x3f;

    /// [Output enable](https://github.com/mithro/ixo-usb-jtag/blob/master/usbjtag.c#L182)
    const _BLASTER_STA_OUT_OE: u8 = 0x20;
    /// [TDI high bit](https://github.com/mithro/ixo-usb-jtag/blob/master/usbjtag.c#L181)
    const BLASTER_STA_OUT_TDI: u8 = 0x10;
    /// [nCS high bit](https://github.com/mithro/ixo-usb-jtag/blob/master/usbjtag.c#L180)
    /// Always 0, which means chip is selected
    const _BLASTER_STA_OUT_NCS: u8 = 0x08;
    /// [nCE high bit](https://github.com/mithro/ixo-usb-jtag/blob/master/usbjtag.c#L179)
    /// Always 0, which means chip is enabled
    const _BLASTER_STA_OUT_NCE: u8 = 0x04;
    /// [TMS high bit](https://github.com/mithro/ixo-usb-jtag/blob/master/usbjtag.c#L178)
    const BLASTER_STA_OUT_TMS: u8 = 0x02;
    /// [TCK high bit](https://github.com/mithro/ixo-usb-jtag/blob/master/usbjtag.c#L177)
    const BLASTER_STA_OUT_TCK: u8 = 0x01;

    // Data from device
    const BLASTER_STA_IN_TDO: u8 = 0x01;
    /// Active serial data out (not used for JTAG)
    const _BLASTER_STA_IN_DATAOUT: u8 = 0x02;

    pub fn new(tdi: TDI, tck: TCK, tms: TMS, tdo: TDO) -> Port<E, TDI, TCK, TMS, TDO> {
        Port {
            tdi,
            tck,
            tms,
            tdo,
            jtag_state: JTAGState::Reset,
            shift_count: 0,
            shift_data: 0,
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
    ) -> Result<(), E> {
        let mut i = 0usize;
        while i < *recv_len && *send_len < send_buf.len() {
            let d = recv_buf[i];
            if self.shift_count == 0 {
                // bit-bang mode (default)
                self.read_en = (d & Self::BLASTER_STA_READ) != 0;
                if d & Self::BLASTER_STA_SHIFT != 0 {
                    // Swap to shift mode for 0 to 63 shifts
                    self.shift_count = d & Self::BLASTER_STA_CNT_MASK;
                    // [Record shift register content and send it to the host](https://github.com/mithro/ixo-usb-jtag/blob/master/usbjtag.c#L199)
                    if self.read_en {
                        send_buf[*send_len] = self.shift_data;
                        *send_len += 1;
                    }
                } else {
                    self.set_state(d)?;
                    if self.read_en {
                        send_buf[*send_len] = self.get_state()?;
                        *send_len += 1;
                    }
                }
            } else {
                // shift-mode
                if self.read_en {
                    self.shift_data = d;
                    self.shift_io()?;
                    send_buf[*send_len] = self.shift_data;
                    *send_len += 1;
                } else {
                    self.shift_data = d;
                    self.shift_out()?;
                }
                self.shift_count -= 1;
            }
            i += 1;
        }
        if i != 0 {
            recv_buf.copy_within(i..*recv_len, 0);
        }
        Ok(())
    }

    fn advance(&mut self, mode: bool) {
        self.jtag_state = self.jtag_state.advance(mode);
    }

    pub fn set_state(&mut self, d: u8) -> Result<(), E> {
        if (d & Self::BLASTER_STA_OUT_TDI) >> 4 != 0 {
            self.tdi.set_high()?;
        } else {
            self.tdi.set_low()?;
        }
        let tms = ((d & Self::BLASTER_STA_OUT_TMS) >> 1) != 0;
        if tms {
            self.tms.set_high()?;
        } else {
            self.tms.set_low()?;
        }
        let clk = d & Self::BLASTER_STA_OUT_TCK != 0;
        if self.got_clock && !clk {
            self.advance(tms);
            self.got_clock = false;
        }
        if clk {
            self.got_clock = true;
            self.tck.set_high()
        } else {
            self.tck.set_low()
        }
    }

    /// [Record the state of TDO and nSTATUS](https://github.com/mithro/ixo-usb-jtag/blob/master/usbjtag.c#L184)
    pub fn get_state(&mut self) -> Result<u8, E> {
        let mut d = 0u8;
        if self.tdo.is_high()? {
            d |= Self::BLASTER_STA_IN_TDO;
        }
        Ok(d)
    }

    pub fn reset(&mut self) -> Result<(), E> {
        self.shift_count = 0;
        self.read_en = false;
        self.got_clock = false;
        let res = self.tdi.set_low();
        if res.is_err() {
            self.jtag_state = JTAGState::Undefined;
            return res;
        }
        let res = self.tck.set_low();
        if res.is_err() {
            self.jtag_state = JTAGState::Undefined;
            return res;
        }
        let res = self.tms.set_low();
        if res.is_err() {
            self.jtag_state = JTAGState::Undefined;
            return res;
        }
        self.jtag_state = JTAGState::Reset;
        Ok(())
    }

    fn shift_out(&mut self) -> Result<(), E> {
        for _i in 0..8 {
            if self.shift_data & 1 != 0 {
                self.tdi.set_high()?;
            } else {
                self.tdi.set_low()?;
            }
            self.tck.set_high()?;
            self.shift_data = self.shift_data.rotate_right(1);
            self.tck.set_low()?;
        }
        Ok(())
    }

    fn shift_io(&mut self) -> Result<(), E> {
        for _i in 0..8 {
            if self.shift_data & 1 != 0 {
                self.tdi.set_high()?;
            } else {
                self.tdi.set_low()?;
            }
            let din = self.tdo.is_high()?;
            self.tck.set_high()?;
            self.shift_data = self.shift_data.rotate_right(1);
            if din {
                self.shift_data |= 0b1000_0000u8;
            }
            self.tck.set_low()?;
        }
        Ok(())
    }
}
