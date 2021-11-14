use super::bus::AccessType;
use crate::schedule::Timestamp;

#[derive(Clone, Copy, Debug)]
pub struct Timer {
    enabled: bool,
    cycle_shift: u8,
    internal_counter: u8,
    up_counter: u8,
    internal_counter_max: u16,
    last_update: Timestamp,
}

impl Timer {
    pub(crate) const fn new(cycle_shift: u8) -> Self {
        Timer {
            enabled: false,
            cycle_shift,
            internal_counter: 0,
            up_counter: 0,
            internal_counter_max: 0xFF,
            last_update: 0,
        }
    }

    pub(super) fn set_enabled(&mut self, value: bool, time: Timestamp) {
        // The counters are reset on a rising edge of the enable bit according to bsnes:
        // https://github.com/bsnes-emu/bsnes/blob/master/bsnes/sfc/smp/io.cpp#L96
        let was_enabled = self.enabled;
        self.enabled = value;
        if value && !was_enabled {
            self.internal_counter = 0;
            self.up_counter = 0;
            self.last_update = time;
        }
    }

    #[inline]
    pub fn cycle_shift(&self) -> u8 {
        self.cycle_shift
    }

    fn update(&mut self, time: Timestamp) {
        if !self.enabled {
            return;
        }
        let elapsed = (time >> self.cycle_shift) - (self.last_update >> self.cycle_shift);
        self.last_update = time;
        let new_internal_counter = self.internal_counter as Timestamp + elapsed;
        let internal_counter_max = self.internal_counter_max as Timestamp;
        self.internal_counter = (new_internal_counter % internal_counter_max) as u8;
        self.up_counter = self
            .up_counter
            .wrapping_add((new_internal_counter / internal_counter_max) as u8)
            & 0xF;
    }

    #[inline]
    pub fn internal_counter(&mut self, time: Timestamp) -> u8 {
        self.update(time);
        self.internal_counter
    }

    #[inline]
    pub fn internal_counter_max(&self) -> u8 {
        self.internal_counter_max as u8
    }

    #[inline]
    pub fn set_internal_counter_max(&mut self, value: u8, time: Timestamp) {
        self.update(time);
        self.internal_counter_max = if value == 0 { 256 } else { value as u16 };
    }

    #[inline]
    pub fn up_counter(&mut self, time: Timestamp) -> u8 {
        self.update(time);
        self.up_counter
    }

    #[inline]
    pub fn read_up_counter<A: AccessType>(&mut self, time: Timestamp) -> u8 {
        self.update(time);
        let result = self.up_counter;
        if A::SIDE_EFFECTS {
            self.up_counter = 0;
        }
        result
    }
}
