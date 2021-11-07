use crate::utils::{
    bounded_int,
    schedule::{self, RawTimestamp},
};

pub type Timestamp = RawTimestamp;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Frame,
}

impl Default for Event {
    fn default() -> Self {
        Event::Frame
    }
}

pub mod event_slots {
    use crate::utils::def_event_slots;
    def_event_slots!(super::EventSlotIndex, FRAME);
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
    pub(crate) cur_timestamp: Timestamp,
    pub(crate) target_timestamp: Timestamp,
    pub(crate) schedule: schedule::Schedule<Timestamp, Event, EventSlotIndex, EVENT_SLOTS>,
}

impl Schedule {
    pub(super) fn new() -> Self {
        Schedule {
            cur_timestamp: 0,
            target_timestamp: 0,
            schedule: schedule::Schedule::new(),
        }
    }

    #[inline]
    pub fn cur_timestamp(&self) -> Timestamp {
        self.cur_timestamp
    }

    #[inline]
    pub fn target_timestamp(&self) -> Timestamp {
        self.target_timestamp
    }

    #[inline]
    pub fn schedule(&self) -> &schedule::Schedule<Timestamp, Event, EventSlotIndex, EVENT_SLOTS> {
        &self.schedule
    }

    pub(crate) fn schedule_event(&mut self, slot_index: EventSlotIndex, time: Timestamp) {
        self.schedule.schedule(slot_index, time);
        if time < self.target_timestamp {
            self.target_timestamp = time;
        }
    }

    pub(crate) fn forward_to_target(&mut self) {
        self.cur_timestamp = self.target_timestamp;
    }
}
