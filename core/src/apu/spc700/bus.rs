use super::Control;
use crate::apu::Apu;

pub trait AccessType {
    const SIDE_EFFECTS: bool;
    const LOG: bool;
}

pub struct ApuAccess;

impl AccessType for ApuAccess {
    const SIDE_EFFECTS: bool = true;
    const LOG: bool = true;
}

pub struct ApuDummyAccess;

impl AccessType for ApuDummyAccess {
    const SIDE_EFFECTS: bool = true;
    const LOG: bool = false;
}

pub struct DebugAccess;

impl AccessType for DebugAccess {
    const SIDE_EFFECTS: bool = false;
    const LOG: bool = false;
}

pub fn read<A: AccessType>(apu: &mut Apu, addr: u16) -> u8 {
    match addr {
        0x00F0..=0x00F1 | 0x00FA..=0x00FC => {
            #[cfg(feature = "log")]
            if A::LOG {
                slog::warn!(
                    apu.spc700.logger,
                    "Write-only I/O reg read @ {:#06X} @ {:#06X}",
                    addr,
                    apu.spc700.regs.pc,
                );
            }
            0
        }
        0x00F2 => apu.spc700.dsp_reg_index,
        0x00F4..=0x00F7 => apu.spc700.cpu_to_apu[addr as usize & 3],
        0x00FD => apu.spc700.timers[0].read_up_counter::<A>(apu.spc700.cur_timestamp),
        0x00FE => apu.spc700.timers[1].read_up_counter::<A>(apu.spc700.cur_timestamp),
        0x00FF => apu.spc700.timers[2].read_up_counter::<A>(apu.spc700.cur_timestamp),
        0xFFC0..=0xFFFF if apu.spc700.control.bootrom_enabled() => {
            apu.spc700.ipl_rom[addr as usize & 0x3F]
        }
        _ => apu.spc700.memory[addr as usize],
    }
}

pub fn write<A: AccessType>(apu: &mut Apu, addr: u16, value: u8) {
    match addr {
        0x00F0 =>
        {
            #[cfg(feature = "log")]
            if A::LOG {
                slog::warn!(
                    apu.spc700.logger,
                    "TEST I/O reg write: {:#04X} @ {:#06X}",
                    value,
                    apu.spc700.regs.pc,
                );
            }
        }
        0x00F1 => apu
            .spc700
            .set_control(Control(value), apu.spc700.cur_timestamp),
        0x00F2 => apu.spc700.dsp_reg_index = value,
        0x00F4..=0x00F7 => apu.spc700.apu_to_cpu[addr as usize & 3] = value,
        0x00FA => apu.spc700.timers[0].set_internal_counter_max(value, apu.spc700.cur_timestamp),
        0x00FB => apu.spc700.timers[1].set_internal_counter_max(value, apu.spc700.cur_timestamp),
        0x00FC => apu.spc700.timers[2].set_internal_counter_max(value, apu.spc700.cur_timestamp),
        0x00FD..=0x00FF =>
        {
            #[cfg(feature = "log")]
            if A::LOG {
                slog::warn!(
                    apu.spc700.logger,
                    "Read-only I/O reg write @ {:#06X}: {:#04X} @ {:#06X}",
                    addr,
                    value,
                    apu.spc700.regs.pc,
                );
            }
        }
        _ => apu.spc700.memory[addr as usize] = value,
    }
}
