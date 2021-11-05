use crate::emu::Emu;

pub mod regs;

use regs::Regs;

pub struct Cpu {
    #[cfg(feature = "log")]
    logger: slog::Logger,
    pub regs: Regs,
}

impl Cpu {
    pub(crate) fn new(#[cfg(feature = "log")] logger: slog::Logger) -> Self {
        Cpu {
            #[cfg(feature = "log")]
            logger,
            regs: Regs::new(),
        }
    }

    pub(crate) fn run_frame(_emu: &mut Emu) {
        // TODO
    }
}
