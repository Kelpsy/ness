use crate::{
    ppu,
    utils::{
        bounded_int,
        schedule::{self, RawTimestamp},
    },
};

pub type Timestamp = RawTimestamp;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Ppu(ppu::Event),
    HvIrq,
}

impl Default for Event {
    fn default() -> Self {
        Event::Ppu(ppu::Event::StartHDraw)
    }
}

pub mod event_slots {
    use crate::utils::def_event_slots;
    def_event_slots!(super::EventSlotIndex, PPU, PPU_OTHER, HV_IRQ);
}
pub const EVENT_SLOTS: usize = event_slots::LEN;

bounded_int!(pub struct EventSlotIndex(u8), max EVENT_SLOTS as u8);

impl From<usize> for EventSlotIndex {
    #[inline]
    fn from(v: usize) -> Self {
        assert!(v < event_slots::LEN);
        unsafe { Self::new_unchecked(v as u8) }
    }
}

impl From<EventSlotIndex> for usize {
    #[inline]
    fn from(v: EventSlotIndex) -> Self {
        v.get() as usize
    }
}

pub struct Schedule {
    pub(crate) cur_time: Timestamp,
    pub(crate) target_time: Timestamp,
    pub(crate) schedule: schedule::Schedule<Timestamp, Event, EventSlotIndex, EVENT_SLOTS>,
}

impl Schedule {
    pub(super) fn new() -> Self {
        Schedule {
            cur_time: 0,
            target_time: 0,
            schedule: schedule::Schedule::new(),
        }
    }

    #[inline]
    pub fn cur_time(&self) -> Timestamp {
        self.cur_time
    }

    #[inline]
    pub fn target_time(&self) -> Timestamp {
        self.target_time
    }

    #[inline]
    pub fn schedule(&self) -> &schedule::Schedule<Timestamp, Event, EventSlotIndex, EVENT_SLOTS> {
        &self.schedule
    }

    pub(crate) fn set_event(&mut self, slot_index: EventSlotIndex, event: Event) {
        self.schedule.set_event(slot_index, event);
    }

    pub(crate) fn schedule_event(&mut self, slot_index: EventSlotIndex, time: Timestamp) {
        self.schedule.schedule(slot_index, time);
        if time < self.target_time {
            self.target_time = time;
        }
    }

    pub(crate) fn cancel_event(&mut self, slot_index: EventSlotIndex) {
        self.schedule.cancel(slot_index);
    }

    pub(crate) fn pop_pending_event(&mut self) -> Option<(Event, Timestamp)> {
        self.schedule.pop_pending_event(self.cur_time)
    }

    pub(crate) fn forward_to_target(&mut self) {
        self.cur_time = self.target_time;
    }
}
