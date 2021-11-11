pub mod bus;
pub(super) mod interpreter;
pub mod regs;
mod timers;
pub use timers::Timer;

use crate::{
    schedule::Timestamp,
    utils::{bitfield_debug, zeroed_box, Bytes},
};
use regs::Regs;

bitfield_debug! {
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct Control(pub u8) {
        pub timers_enable_mask: u8 @ 0..=2,
        pub reset_ports_01: bool @ 4,
        pub reset_ports_23: bool @ 5,
        pub bootrom_enabled: bool @ 7,
    }
}

pub struct Spc700 {
    #[cfg(feature = "log")]
    logger: slog::Logger,
    pub cur_timestamp: Timestamp,
    pub regs: Regs,
    ipl_rom: Bytes<0x40>,
    pub memory: Box<Bytes<0x1_0000>>,
    control: Control,
    pub timers: [Timer; 3],
    pub cpu_to_apu: [u8; 4],
    pub apu_to_cpu: [u8; 4],
    dsp_reg_index: u8,
}

impl Spc700 {
    pub(crate) fn new(#[cfg(feature = "log")] logger: slog::Logger) -> Self {
        let mut memory = zeroed_box::<Bytes<0x1_0000>>();
        for base in (0..0x1_0000).step_by(0x40) {
            memory[base + 0x20..base + 0x40].fill(0xFF);
        }
        Spc700 {
            #[cfg(feature = "log")]
            logger,
            cur_timestamp: 0,
            regs: Regs::new(),
            ipl_rom: Bytes::new(*include_bytes!("spc700/ipl.rom")),
            memory,
            control: Control(0x80),
            timers: [Timer::new(7), Timer::new(7), Timer::new(4)],
            cpu_to_apu: [0; 4],
            apu_to_cpu: [0; 4],
            dsp_reg_index: 0,
        }
    }

    #[inline]
    pub fn ipl_rom(&self) -> &Bytes<0x40> {
        &self.ipl_rom
    }

    #[inline]
    pub fn control(&self) -> Control {
        self.control
    }

    pub fn set_control(&mut self, value: Control, time: Timestamp) {
        self.control = value;
        if value.reset_ports_01() {
            self.cpu_to_apu[0..2].fill(0);
        }
        if value.reset_ports_23() {
            self.cpu_to_apu[2..4].fill(0);
        }
        let timers_enable_mask = value.timers_enable_mask();
        for (i, timer) in self.timers.iter_mut().enumerate() {
            timer.set_enabled(timers_enable_mask & 1 << i != 0, time);
        }
    }
}
