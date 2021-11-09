pub mod empty;
pub mod joypad;

use crate::schedule::{self, event_slots, Schedule, Timestamp};
use empty::Empty;
use joypad::Joypad;
use std::any::Any;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    StartAutoRead,
    EndAutoRead,
}

pub trait Device: Any {
    fn as_any(&mut self) -> &mut dyn Any;
    fn auto_read(&mut self) -> u16;
}

pub struct Controllers {
    last_auto_read: Option<Timestamp>,
    joypad_auto_read_enabled: bool,
    joypad_auto_read_busy: bool,
    pub devices: [Box<dyn Device>; 4],
    pub auto_read_results: [u16; 4],
}

impl Controllers {
    pub(crate) fn new(schedule: &mut Schedule) -> Self {
        schedule.set_event(
            event_slots::CONTROLLERS,
            schedule::Event::Controllers(Event::StartAutoRead),
        );
        Controllers {
            last_auto_read: None,
            joypad_auto_read_enabled: false,
            joypad_auto_read_busy: false,
            devices: [
                Box::new(Joypad::new()),
                Box::new(Empty::new()),
                Box::new(Empty::new()),
                Box::new(Empty::new()),
            ],
            auto_read_results: [0; 4],
        }
    }

    pub(crate) fn handle_event(&mut self, event: Event, time: Timestamp, schedule: &mut Schedule) {
        match event {
            Event::StartAutoRead => {
                if self.joypad_auto_read_enabled {
                    self.joypad_auto_read_busy = true;
                }
                for i in 0..4 {
                    self.auto_read_results[i] = self.devices[i].auto_read();
                }
                schedule.set_event(
                    event_slots::CONTROLLERS,
                    schedule::Event::Controllers(Event::EndAutoRead),
                );
                schedule.schedule_event(event_slots::CONTROLLERS, time + 4224);
            }
            Event::EndAutoRead => {
                self.joypad_auto_read_busy = false;
                schedule.set_event(
                    event_slots::CONTROLLERS,
                    schedule::Event::Controllers(Event::StartAutoRead),
                );
            }
        }
    }

    pub(crate) fn last_auto_read(&self) -> Option<Timestamp> {
        self.last_auto_read
    }

    #[inline]
    pub fn joypad_auto_read_enabled(&self) -> bool {
        self.joypad_auto_read_enabled
    }

    #[inline]
    pub fn set_joypad_auto_read_enabled(&mut self, value: bool) {
        self.joypad_auto_read_enabled = value;
    }

    #[inline]
    pub fn joypad_auto_read_busy(&self) -> bool {
        self.joypad_auto_read_busy
    }
}
