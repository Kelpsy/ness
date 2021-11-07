pub mod schedule;

use crate::{
    cart::Cart,
    cpu::Cpu,
    utils::{zeroed_box, Bytes},
    Model,
};
use schedule::{event_slots, Event, Schedule};

pub struct Emu {
    pub cpu: Cpu,
    pub cart: Cart,
    pub schedule: Schedule,
    pub wram: Box<Bytes<0x2_0000>>,
}

impl Emu {
    pub fn new(_model: Model, cart: Cart, #[cfg(feature = "log")] logger: &slog::Logger) -> Self {
        let mut emu = Emu {
            cpu: Cpu::new(
                #[cfg(feature = "log")]
                logger.new(slog::o!("cpu" => "")),
            ),
            cart,
            schedule: Schedule::new(),
            wram: zeroed_box(),
        };
        emu.soft_reset();
        emu
    }

    pub fn soft_reset(&mut self) {
        // TODO: Reset other components
        Cpu::soft_reset(self);
    }

    pub fn run_frame(&mut self) {
        self.schedule
            .schedule
            .set_event(event_slots::FRAME, Event::Frame);
        self.schedule
            .schedule_event(event_slots::FRAME, self.schedule.cur_time + 70_000);
        loop {
            Cpu::run_until_next_event(self);
            #[allow(clippy::never_loop)] // TODO: Remove
            while let Some((event, _)) = self.schedule.pop_pending_event() {
                match event {
                    Event::Frame => return,
                }
            }
        }
    }
}
