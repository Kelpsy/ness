pub mod dsp;
pub mod spc700;

use crate::schedule::Timestamp;
use dsp::Dsp;
use spc700::Spc700;

pub struct Apu {
    pub spc700: Spc700,
    pub dsp: Dsp,
}

impl Apu {
    pub(crate) fn new(#[cfg(feature = "log")] logger: &slog::Logger) -> Self {
        Apu {
            spc700: Spc700::new(
                #[cfg(feature = "log")]
                logger.new(slog::o!("spc700" => "")),
            ),
            dsp: Dsp::new(
                #[cfg(feature = "log")]
                logger.new(slog::o!("dsp" => "")),
            ),
        }
    }

    pub(crate) fn run(&mut self, end_main_timestamp: Timestamp) {
        let end_timestamp = end_main_timestamp * 102400 / 2147727; // TODO: Something less hacky?
        spc700::interpreter::run(self, end_timestamp);
    }
}
