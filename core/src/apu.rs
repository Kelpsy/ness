pub mod dsp;
pub mod spc700;

use crate::{
    schedule::{event_slots, Event, Schedule, Timestamp},
    Model,
};
use dsp::Dsp;
use spc700::Spc700;

pub struct Apu {
    pub spc700: Spc700,
    pub dsp: Dsp,
    model: Model,
    dsp_timestamp: Timestamp,
}

impl Apu {
    pub(crate) fn new(
        backend: Box<dyn dsp::Backend>,
        sample_chunk_len: usize,
        model: Model,
        schedule: &mut Schedule,
        #[cfg(feature = "log")] logger: &slog::Logger,
    ) -> Self {
        schedule.set_event(event_slots::APU, Event::UpdateApu);
        schedule.schedule_event(event_slots::APU, 0);
        Apu {
            spc700: Spc700::new(
                #[cfg(feature = "log")]
                logger.new(slog::o!("spc700" => "")),
            ),
            dsp: Dsp::new(
                backend,
                sample_chunk_len,
                #[cfg(feature = "log")]
                logger.new(slog::o!("dsp" => "")),
            ),
            model,
            dsp_timestamp: 0,
        }
    }

    pub(crate) fn handle_update(&mut self, time: Timestamp, schedule: &mut Schedule) {
        self.run(time);
        Dsp::output_sample(self);
        self.dsp_timestamp += 1;
        schedule.schedule_event(
            event_slots::APU,
            if self.model == Model::Pal {
                self.dsp_timestamp as u128 * 17734475 / 32000
            } else {
                self.dsp_timestamp as u128 * 2147727 / 3200
            } as Timestamp,
        );
    }

    pub(crate) fn soft_reset(&mut self) {
        // TODO: Soft-reset DSP
        Spc700::soft_reset(self);
    }

    pub(crate) fn run(&mut self, end_main_timestamp: Timestamp) {
        // TODO: Something less hacky?
        let end_timestamp = if self.model == Model::Pal {
            end_main_timestamp as u128 * 1024000 / 17734475
        } else {
            end_main_timestamp as u128 * 102400 / 2147727
        } as Timestamp;
        spc700::interpreter::run(self, end_timestamp);
    }
}
