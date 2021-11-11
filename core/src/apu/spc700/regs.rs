use crate::utils::bitfield_debug;

bitfield_debug!(
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct Psw(pub u8) {
        pub carry: bool @ 0,
        pub zero: bool @ 1,
        pub irqs_enabled: bool @ 2,
        pub half_carry: bool @ 3,
        pub break_flag: bool @ 4,
        pub direct_page: bool @ 5,
        pub overflow: bool @ 6,
        pub negative: bool @ 7,
    }
);

#[derive(Clone, Debug)]
pub struct Regs {
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub sp: u8,
    pub pc: u16,
    pub(crate) psw: Psw,
    direct_page_base: u16,
}

impl Regs {
    pub(crate) fn new() -> Self {
        Regs {
            a: 0,
            x: 0,
            y: 0,
            sp: 0xFF,
            pc: 0xFFC0,
            psw: Psw(0),
            direct_page_base: 0,
        }
    }

    #[inline]
    pub fn ya(&self) -> u16 {
        (self.y as u16) << 8 | self.a as u16
    }

    #[inline]
    pub fn set_ya(&mut self, value: u16) {
        self.a = value as u8;
        self.y = (value >> 8) as u8;
    }

    #[inline]
    pub fn psw(&self) -> Psw {
        self.psw
    }

    #[inline]
    pub fn set_psw(&mut self, value: Psw) {
        self.psw = value;
        self.direct_page_base = (self.psw.direct_page() as u16) << 8;
    }

    pub(crate) fn direct_page_base(&self) -> u16 {
        self.direct_page_base
    }
}
