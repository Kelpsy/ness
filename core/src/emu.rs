use crate::{
    cart::Cart,
    controllers::Controllers,
    cpu::Cpu,
    ppu::Ppu,
    schedule::{Event, Schedule},
    Model, Wram,
};

pub struct Emu {
    pub cpu: Cpu,
    pub wram: Wram,
    pub schedule: Schedule,
    pub ppu: Ppu,
    pub cart: Cart,
    pub controllers: Controllers,
}

impl Emu {
    pub fn new(model: Model, cart: Cart, #[cfg(feature = "log")] logger: &slog::Logger) -> Self {
        let mut schedule = Schedule::new();
        let mut emu = Emu {
            cpu: Cpu::new(
                #[cfg(feature = "log")]
                logger.new(slog::o!("cpu" => "")),
            ),
            wram: Wram::new(),
            ppu: Ppu::new(model, &mut schedule),
            cart,
            controllers: Controllers::new(&mut schedule),
            schedule,
        };
        emu.soft_reset();
        emu
    }

    pub fn soft_reset(&mut self) {
        // TODO: Reset other components
        Cpu::soft_reset(self);
    }

    pub fn run_frame(&mut self) {
        while !self.ppu.frame_finished {
            Cpu::run_until_next_event(self);
            #[allow(clippy::never_loop)] // TODO: Remove
            while let Some((event, time)) = self.schedule.pop_pending_event() {
                match event {
                    Event::Ppu(event) => Ppu::handle_event(self, event, time),
                    Event::HvIrq => self
                        .ppu
                        .counters
                        .handle_hv_irq_triggered(&mut self.cpu.irqs, &mut self.schedule),
                    Event::Controllers(event) => {
                        self.controllers
                            .handle_event(event, time, &mut self.schedule)
                    }
                }
            }
        }
        self.ppu.frame_finished = false;
    }
}
