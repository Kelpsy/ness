use crate::{
    cpu::bus::AccessType,
    utils::{zeroed_box, Bytes},
};

mod bounded {
    use crate::utils::bounded_int;
    bounded_int!(pub(super) struct Address(u32), max 0x1_FFFF);
}
use bounded::Address;

pub struct Wram {
    pub contents: Box<Bytes<0x20000>>,
    cur_addr: Address,
}

impl Wram {
    pub(crate) fn new() -> Self {
        Wram {
            contents: zeroed_box(),
            cur_addr: Address::new(0),
        }
    }

    #[inline]
    pub fn cur_addr(&self) -> u32 {
        self.cur_addr.get()
    }

    #[inline]
    pub fn set_addr(&mut self, value: u32) {
        self.cur_addr = Address::new(value & 0x1_FFFF);
    }

    #[inline]
    pub fn read_data<A: AccessType>(&mut self) -> u8 {
        let result = self.contents[self.cur_addr.get() as usize];
        if A::SIDE_EFFECTS {
            self.cur_addr = Address::new((self.cur_addr.get() + 1) & 0x1_FFFF);
        }
        result
    }

    #[inline]
    pub fn write_data(&mut self, value: u8) {
        self.contents[self.cur_addr.get() as usize] = value;
        self.cur_addr = Address::new((self.cur_addr.get() + 1) & 0x1_FFFF);
    }
}
