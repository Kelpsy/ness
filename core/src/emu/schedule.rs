use crate::utils::{
    bounded_int,
    schedule::{self, RawTimestamp},
};

pub type Timestamp = RawTimestamp;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Todo,
}

impl Default for Event {
    fn default() -> Self {
        Event::Todo
    }
}

pub mod event_slots {
    use crate::utils::def_event_slots;
    def_event_slots!(super::EventSlotIndex,);
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

pub type Schedule = schedule::Schedule<Timestamp, Event, EventSlotIndex, EVENT_SLOTS>;
