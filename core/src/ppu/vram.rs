use super::Ppu;
use crate::{
    cpu::bus::AccessType,
    utils::{bitfield_debug, zeroed_box, Bytes},
};

bitfield_debug! {
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct IncrementControl(pub u8) {
        pub incr_step: u8 @ 0..=1,
        pub addr_translation: u8 @ 2..=3,
        pub incr_after_high_byte_access: bool @ 7,
    }
}

pub struct Vram {
    pub contents: Box<Bytes<0x1_0000>>,
    increment_control: IncrementControl,
    addr_increment: u8,
    read_latch: u16,
    cpu_written_addr: u16,
    cur_word_addr: u16,
}

impl Vram {
    pub(crate) fn new() -> Self {
        Vram {
            contents: zeroed_box(),
            increment_control: IncrementControl(0),
            addr_increment: 1,
            cpu_written_addr: 0,
            cur_word_addr: 0,
            read_latch: 0,
        }
    }

    #[inline]
    pub fn increment_control(&self) -> IncrementControl {
        self.increment_control
    }

    #[inline]
    pub fn set_increment_control(&mut self, value: IncrementControl) {
        self.increment_control = value;
        self.addr_increment = match self.increment_control.incr_step() {
            0 => 1,
            1 => 32,
            _ => 128,
        };
    }

    #[inline]
    pub fn read_latch(&self) -> u16 {
        self.read_latch
    }

    #[inline]
    pub fn cur_word_addr(&self) -> u16 {
        self.cur_word_addr
    }

    fn reset_cur_word_addr(&mut self) {
        self.cur_word_addr = self.cpu_written_addr;
        self.read_latch = self.contents.read_le((self.cur_word_addr << 1) as usize);
    }

    fn translated_cur_word_addr(&self) -> u16 {
        match self.increment_control.addr_translation() {
            0 => self.cur_word_addr,
            1 => (self.cur_word_addr & 0xFF00) | (self.cur_word_addr as u8).rotate_left(3) as u16,
            2 => {
                (self.cur_word_addr & 0xFE00)
                    | (self.cur_word_addr << 3 & 0x1F8)
                    | (self.cur_word_addr >> 6 & 7)
            }
            _ => {
                (self.cur_word_addr & 0xFC00)
                    | (self.cur_word_addr << 3 & 0x3F8)
                    | (self.cur_word_addr >> 7 & 7)
            }
        }
    }

    #[inline]
    pub fn set_addr_low(&mut self, value: u8) {
        self.cpu_written_addr = (self.cpu_written_addr & 0xFF00) | value as u16;
        self.reset_cur_word_addr();
    }

    #[inline]
    pub fn set_addr_high(&mut self, value: u8) {
        self.cpu_written_addr = (self.cpu_written_addr & 0xFF) | (value as u16) << 8;
        self.reset_cur_word_addr();
    }
}

impl Ppu {
    pub fn read_vram_low<A: AccessType>(&mut self) -> u8 {
        let result = self.vram.read_latch as u8;
        if A::SIDE_EFFECTS {
            if !self.vram.increment_control.incr_after_high_byte_access() {
                self.vram.read_latch = self
                    .vram
                    .contents
                    .read_le((self.vram.translated_cur_word_addr() << 1) as usize);
                self.vram.cur_word_addr = self
                    .vram
                    .cur_word_addr
                    .wrapping_add(self.vram.addr_increment as u16);
            }
            self.ppu1_mdr = result;
        }
        result
    }

    pub fn read_vram_high<A: AccessType>(&mut self) -> u8 {
        let result = (self.vram.read_latch >> 8) as u8;
        if A::SIDE_EFFECTS {
            if self.vram.increment_control.incr_after_high_byte_access() {
                self.vram.read_latch = self
                    .vram
                    .contents
                    .read_le((self.vram.translated_cur_word_addr() << 1) as usize);
                self.vram.cur_word_addr = self
                    .vram
                    .cur_word_addr
                    .wrapping_add(self.vram.addr_increment as u16);
            }
            self.ppu1_mdr = result;
        }
        result
    }

    pub fn write_vram_low(&mut self, value: u8) {
        let translated_addr = self.vram.translated_cur_word_addr();
        self.vram.contents[(translated_addr << 1) as usize] = value;
        if !self.vram.increment_control.incr_after_high_byte_access() {
            self.vram.cur_word_addr = self
                .vram
                .cur_word_addr
                .wrapping_add(self.vram.addr_increment as u16);
        }
    }

    pub fn write_vram_high(&mut self, value: u8) {
        let translated_addr = self.vram.translated_cur_word_addr();
        self.vram.contents[(translated_addr << 1) as usize | 1] = value;
        if self.vram.increment_control.incr_after_high_byte_access() {
            self.vram.cur_word_addr = self
                .vram
                .cur_word_addr
                .wrapping_add(self.vram.addr_increment as u16);
        }
    }
}
