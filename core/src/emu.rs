use crate::{
    apu::{dsp, Apu},
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
    pub apu: Apu,
    pub ppu: Ppu,
    pub cart: Cart,
    pub controllers: Controllers,
}

impl Emu {
    pub fn new(
        model: Model,
        cart: Cart,
        audio_backend: Box<dyn dsp::Backend>,
        audio_sample_chunk_len: usize,
        #[cfg(feature = "log")] logger: &slog::Logger,
    ) -> Self {
        let mut schedule = Schedule::new();
        let mut emu = Emu {
            cpu: Cpu::new(
                #[cfg(feature = "log")]
                logger.new(slog::o!("cpu" => "")),
            ),
            wram: Wram::new(),
            apu: Apu::new(
                audio_backend,
                audio_sample_chunk_len,
                model,
                &mut schedule,
                #[cfg(feature = "log")]
                logger,
            ),
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
        self.apu.soft_reset();
        Cpu::soft_reset(self);
    }

    pub fn run_frame(&mut self) {
        while !self.ppu.frame_finished {
            Cpu::run_until_next_event(self);
            self.schedule.last_poll_time = self.schedule.cur_time;
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
                    Event::UpdateApu => self.apu.handle_update(time, &mut self.schedule),
                }
            }
        }
        self.ppu.frame_finished = false;
    }
}
