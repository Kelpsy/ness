use crate::{utils::BoxedByteSlice, Model};

pub struct Emu {}

impl Emu {
    pub fn new(
        _model: Model,
        _rom: BoxedByteSlice,
        #[cfg(feature = "log")] _logger: &slog::Logger,
    ) -> Self {
        Emu {}
    }

    pub fn run_frame(&mut self) {

    }
}
