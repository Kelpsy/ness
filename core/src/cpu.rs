use crate::emu::{schedule::Timestamp, Emu};

pub mod regs;
pub mod bus;

mod common;
mod interpreter;

use regs::Regs;

pub struct Cpu {
    #[cfg(feature = "log")]
    logger: slog::Logger,
    pub regs: Regs,
    pub stopped: bool,
    pub cur_timestamp: Timestamp,
}

impl Cpu {
    pub(crate) fn new(#[cfg(feature = "log")] logger: slog::Logger) -> Self {
        Cpu {
            #[cfg(feature = "log")]
            logger,
            regs: Regs::new(),
            stopped: false,
            cur_timestamp: 0,
        }
    }

    pub(crate) fn soft_reset(emu: &mut Emu) {
        interpreter::soft_reset(emu);
    }

    pub(crate) fn run_until_next_event(emu: &mut Emu) {
        interpreter::run_until_next_event(emu)
    }
}
