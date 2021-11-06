pub mod schedule;

use crate::{cart::Cart, cpu::Cpu, Model};
use schedule::Schedule;

pub struct Emu {
    pub cpu: Cpu,
    pub cart: Cart,
    pub schedule: Schedule,
}

impl Emu {
    pub fn new(_model: Model, cart: Cart, #[cfg(feature = "log")] logger: &slog::Logger) -> Self {
        Emu {
            cpu: Cpu::new(
                #[cfg(feature = "log")]
                logger.new(slog::o!("cpu" => "")),
            ),
            cart,
            schedule: Schedule::new(),
        }
    }

    pub fn run_frame(&mut self) {
        Cpu::run_frame(self);
    }
}
