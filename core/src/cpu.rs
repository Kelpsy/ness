use crate::emu::Emu;

pub mod bus;
pub mod dma;
mod irqs;
pub use irqs::Irqs;
pub mod math;
pub mod regs;

mod common;
#[cfg(feature = "disasm")]
pub mod disasm;
mod interpreter;

use math::Math;
use regs::Regs;

pub struct Cpu {
    #[cfg(feature = "log")]
    logger: slog::Logger,
    pub regs: Regs,
    mdr: u8,
    pub stopped: bool,
    pub irqs: Irqs,
    pub math: Math,
    pub dmac: dma::Controller,
}

impl Cpu {
    pub(crate) fn new(#[cfg(feature = "log")] logger: slog::Logger) -> Self {
        Cpu {
            #[cfg(feature = "log")]
            logger,
            regs: Regs::new(),
            mdr: 0,
            stopped: false,
            irqs: Irqs::new(),
            math: Math::new(),
            dmac: dma::Controller::new(),
        }
    }

    #[inline]
    pub fn mdr(&self) -> u8 {
        self.mdr
    }

    pub(crate) fn soft_reset(emu: &mut Emu) {
        interpreter::soft_reset(emu);
    }

    #[inline]
    pub(crate) fn run_until_next_event(emu: &mut Emu) {
        interpreter::run_until_next_event(emu)
    }
}
