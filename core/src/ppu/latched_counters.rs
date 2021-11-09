use super::Ppu;
use crate::{cpu::bus::AccessType, schedule::Timestamp};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LatchedCounters {
    h_counter: u16,
    v_counter: u16,
    read_high: u8,
}

impl LatchedCounters {
    pub(super) const fn new() -> Self {
        LatchedCounters {
            h_counter: 0x1FF,
            v_counter: 0x1FF,
            read_high: 0,
        }
    }

    #[inline]
    pub fn h_counter(&self) -> u16 {
        self.h_counter
    }

    #[inline]
    pub fn v_counter(&self) -> u16 {
        self.v_counter
    }

    #[inline]
    pub fn read_h_counter<A: AccessType>(&mut self) -> (u8, u8) {
        if self.read_high & 1 != 0 {
            ((self.h_counter >> 8) as u8, 1)
        } else {
            if A::SIDE_EFFECTS {
                self.read_high |= 1;
            }
            (self.h_counter as u8, 0xFF)
        }
    }

    #[inline]
    pub fn read_v_counter<A: AccessType>(&mut self) -> (u8, u8) {
        if self.read_high & 2 != 0 {
            ((self.v_counter >> 8) as u8, 1)
        } else {
            if A::SIDE_EFFECTS {
                self.read_high |= 2;
            }
            (self.v_counter as u8, 0xFF)
        }
    }

    #[inline]
    pub fn h_counter_high_read(&self) -> bool {
        self.read_high & 1 != 0
    }

    #[inline]
    pub fn v_counter_high_read(&self) -> bool {
        self.read_high & 2 != 0
    }
}

impl Ppu {
    #[inline]
    pub fn latch_hv_counters(&mut self, time: Timestamp) {
        self.latched_counters.h_counter = self.counters.h_dot(time);
        self.latched_counters.v_counter = self.counters.v_counter();
        self.latched_counters.read_high = 0;
    }

    #[inline]
    pub fn read_h_latched_counter<A: AccessType>(&mut self) -> u8 {
        let (mut result, mask) = self.latched_counters.read_h_counter::<A>();
        result |= self.ppu2_mdr & !mask;
        if A::SIDE_EFFECTS {
            self.ppu2_mdr = result;
        }
        result
    }

    #[inline]
    pub fn read_v_latched_counter<A: AccessType>(&mut self) -> u8 {
        let (mut result, mask) = self.latched_counters.read_v_counter::<A>();
        result |= self.ppu2_mdr & !mask;
        if A::SIDE_EFFECTS {
            self.ppu2_mdr = result;
        }
        result
    }
}
