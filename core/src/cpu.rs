use crate::emu::{schedule::Timestamp, Emu};

pub mod regs;

mod common;
pub mod disasm;
mod interpreter;

use regs::Regs;

pub struct Cpu {
    #[cfg(feature = "log")]
    logger: slog::Logger,
    pub regs: Regs,
    pub cur_timestamp: Timestamp,
}

impl Cpu {
    pub(crate) fn new(#[cfg(feature = "log")] logger: slog::Logger) -> Self {
        Cpu {
            #[cfg(feature = "log")]
            logger,
            regs: Regs::new(),
            cur_timestamp: 0,
        }
    }

    pub(crate) fn run_frame(emu: &mut Emu) {
        interpreter::run_frame(emu)
    }
}
