use super::Ppu;
use crate::{
    cpu::bus::AccessType,
    utils::{bitfield_debug, zeroed_box},
};

bitfield_debug! {
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct IncrementControl(pub u8) {
        pub incr_step: u8 @ 0..=1,
        pub addr_translation: u8 @ 2..=3,
        pub incr_after_high_byte_access: bool @ 7,
    }
}

pub struct Palette {
    pub contents: Box<[u16; 0x100]>,
    write_latch: u8,
    second_access: bool,
    cur_addr: u8,
}

impl Palette {
    pub(crate) fn new() -> Self {
        Palette {
            contents: zeroed_box(),
            write_latch: 0,
            second_access: false,
            cur_addr: 0,
        }
    }

    #[inline]
    pub fn write_latch(&self) -> u8 {
        self.write_latch
    }

    #[inline]
    pub fn second_access(&self) -> bool {
        self.second_access
    }

    #[inline]
    pub fn cur_addr(&self) -> u8 {
        self.cur_addr
    }

    #[inline]
    pub fn set_word_addr(&mut self, value: u8) {
        self.cur_addr = value;
    }
}

impl Ppu {
    pub fn read_palette<A: AccessType>(&mut self) -> u8 {
        let color = self.palette.contents[self.palette.cur_addr as usize];
        let result = if self.palette.second_access {
            if A::SIDE_EFFECTS {
                self.palette.cur_addr = self.palette.cur_addr.wrapping_add(1);
            }
            ((color >> 8) as u8 & 0x7F) | (self.ppu2_mdr & 0x80)
        } else {
            color as u8
        };
        self.palette.second_access = !self.palette.second_access;
        self.ppu2_mdr = result;
        result
    }

    pub fn write_palette(&mut self, value: u8) {
        if self.palette.second_access {
            self.palette.contents[self.palette.cur_addr as usize] =
                (value as u16 & 0x7F) << 8 | self.palette.write_latch as u16;
            self.palette.cur_addr = self.palette.cur_addr.wrapping_add(1);
        } else {
            self.palette.write_latch = value;
        }
        self.palette.second_access = !self.palette.second_access;
    }
}
