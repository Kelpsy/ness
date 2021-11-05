use crate::{cart::Cart, cpu::Cpu, utils::BoxedByteSlice, Model};

pub struct Emu {
    pub cpu: Cpu,
    pub cart: Cart,
}

impl Emu {
    pub fn new(_model: Model, cart: Cart, #[cfg(feature = "log")] logger: &slog::Logger) -> Self {
        Emu {
            cpu: Cpu::new(
                #[cfg(feature = "log")]
                logger.new(slog::o!("cpu" => "")),
            ),
            cart,
        }
    }

    pub fn run_frame(&mut self) {
        Cpu::run_frame(self);
    }
}
