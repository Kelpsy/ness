use crate::utils::bitfield_debug;

bitfield_debug!(
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct Psw(pub u8) {
        pub carry: bool @ 0,
        pub zero: bool @ 1,
        pub irqs_disabled: bool @ 2,
        pub decimal_mode: bool @ 3,
        pub index_regs_are_8_bit: bool @ 4,
        pub a_is_8_bit: bool @ 5,
        pub overflow: bool @ 6,
        pub negative: bool @ 7,
    }
);

#[derive(Clone, Debug)]
pub struct Regs {
    pub a: u16,
    pub x: u16,
    pub y: u16,
    pub sp: u16,
    pub pc: u16,
    pub direct_page_offset: u16,
    pub(crate) psw: Psw,
    emulation_mode: bool,
    code_bank: u8,
    data_bank: u8,
}

impl Regs {
    pub(crate) fn new() -> Self {
        Regs {
            a: 0,
            x: 0,
            y: 0,
            sp: 0x1FC,
            pc: 0,
            direct_page_offset: 0,
            psw: Psw(0),
            emulation_mode: false,
            code_bank: 0,
            data_bank: 0,
        }
    }

    #[inline]
    pub fn psw(&self) -> Psw {
        self.psw
    }

    #[inline]
    pub fn set_psw(&mut self, value: Psw) {
        self.psw = value;
        if self.psw.index_regs_are_8_bit() {
            self.x &= 0xFF;
            self.y &= 0xFF;
        }
    }

    #[inline]
    pub fn emulation_mode(&self) -> bool {
        self.emulation_mode
    }

    pub fn set_emulation_mode<const RESET: bool>(&mut self, value: bool) {
        if value && !self.emulation_mode && !RESET {
            unimplemented!("Entered unimplemented emulation mode");
        }
        self.emulation_mode = value;
    }

    #[inline]
    pub fn code_bank(&self) -> u8 {
        self.code_bank
    }

    #[inline]
    pub fn set_code_bank(&mut self, value: u8) {
        self.code_bank = value;
    }

    #[inline]
    pub fn data_bank(&self) -> u8 {
        self.data_bank
    }

    #[inline]
    pub fn set_data_bank(&mut self, value: u8) {
        self.data_bank = value;
    }
}
