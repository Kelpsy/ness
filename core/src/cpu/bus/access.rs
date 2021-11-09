use crate::{cpu::dma, emu::Emu, ppu};

pub trait AccessType {
    const NAME: &'static str;
    const IS_DMA: bool;
    const SIDE_EFFECTS: bool;
    const LOG: bool;
}

pub struct CpuAccess;

impl AccessType for CpuAccess {
    const NAME: &'static str = "CPU";
    const IS_DMA: bool = false;
    const SIDE_EFFECTS: bool = true;
    const LOG: bool = true;
}

pub struct DmaAccess;

impl AccessType for DmaAccess {
    const NAME: &'static str = "DMA";
    const IS_DMA: bool = true;
    const SIDE_EFFECTS: bool = true;
    const LOG: bool = true;
}

pub struct DebugCpuAccess;

impl AccessType for DebugCpuAccess {
    const NAME: &'static str = "debug CPU";
    const IS_DMA: bool = false;
    const SIDE_EFFECTS: bool = false;
    const LOG: bool = false;
}

pub struct DebugDmaAccess;

impl AccessType for DebugDmaAccess {
    const NAME: &'static str = "debug DMA";
    const IS_DMA: bool = true;
    const SIDE_EFFECTS: bool = false;
    const LOG: bool = false;
}

fn read_a_io<A: AccessType>(emu: &mut Emu, addr: u32) -> u8 {
    match addr & 0x3FF {
        0x210 => {
            // TODO: Open bus bits
            return emu.ppu.read_nmi_flag::<A>().0;
        }
        0x211 => {
            // TODO: Open bus bits
            return emu
                .ppu
                .counters
                .read_hv_timer_irq_flag::<A>(&mut emu.cpu.irqs, &mut emu.schedule)
                .0;
        }
        0x212 => {
            // TODO: Joypad and open bus bits
            return emu.ppu.hv_status().0;
        }
        0x300..=0x37F => {
            let channel = &emu.cpu.dmac.channels[(addr >> 4 & 7) as usize];
            match addr & 0xF {
                0x0 => return channel.control().0,
                0x1 => return channel.b_addr,
                0x2 => return channel.gp_a_addr_h_table_start_addr as u8,
                0x3 => return (channel.gp_a_addr_h_table_start_addr >> 8) as u8,
                0x4 => return channel.gp_a_bank_h_table_bank,
                0x5 => return channel.gp_byte_counter_h_indirect_addr as u8,
                0x6 => return (channel.gp_byte_counter_h_indirect_addr >> 8) as u8,
                0x7 => return channel.h_indirect_bank,
                0x8 => return channel.h_cur_table_addr as u8,
                0x9 => return (channel.h_cur_table_addr >> 8) as u8,
                0xA => return channel.h_line_counter(),
                0xB | 0xF => return channel.unused,
                _ => {}
            }
        }
        _ => {}
    }

    #[cfg(feature = "log")]
    if A::LOG {
        slog::warn!(
            emu.cpu.logger,
            "Unknown bus A IO {} read @ {:#08X} @ {:#08X}",
            A::NAME,
            addr,
            emu.cpu.regs.code_bank_base() | emu.cpu.regs.pc as u32,
        );
    }
    // TODO: Open bus
    0
}

fn write_a_io<A: AccessType>(emu: &mut Emu, addr: u32, value: u8) {
    match addr & 0x3FF {
        0x200 => {
            // TODO: Joypad control
            return emu.ppu.set_irq_control(
                ppu::IrqControl(value),
                &mut emu.cpu.irqs,
                emu.schedule.cur_time,
                &mut emu.schedule,
            );
        }
        0x207 => {
            return emu.ppu.counters.set_h_timer_value(
                (emu.ppu.counters.h_timer_value() & 0xFF00) | value as u16,
                emu.schedule.cur_time,
                &mut emu.schedule,
            );
        }
        0x208 => {
            return emu.ppu.counters.set_h_timer_value(
                (emu.ppu.counters.h_timer_value() & 0xFF) | (value as u16) << 8,
                emu.schedule.cur_time,
                &mut emu.schedule,
            );
        }
        0x209 => {
            return emu.ppu.counters.set_v_timer_value(
                (emu.ppu.counters.v_timer_value() & 0xFF00) | value as u16,
                emu.schedule.cur_time,
                &mut emu.schedule,
            );
        }
        0x20A => {
            return emu.ppu.counters.set_v_timer_value(
                (emu.ppu.counters.v_timer_value() & 0xFF) | (value as u16) << 8,
                emu.schedule.cur_time,
                &mut emu.schedule,
            );
        }
        0x20B => return emu.cpu.dmac.set_gp_requested(value, &mut emu.schedule),
        0x20C => return emu.cpu.dmac.set_h_enabled(value),
        0x300..=0x37F => {
            let channel = &mut emu.cpu.dmac.channels[(addr >> 4 & 7) as usize];
            match addr & 0xF {
                0x0 => return channel.set_control(dma::ChannelControl(value)),
                0x1 => return channel.b_addr = value,
                0x2 => {
                    return channel.gp_a_addr_h_table_start_addr =
                        (channel.gp_a_addr_h_table_start_addr & 0xFF00) | value as u16
                }
                0x3 => {
                    return channel.gp_a_addr_h_table_start_addr =
                        (channel.gp_a_addr_h_table_start_addr & 0x00FF) | (value as u16) << 8
                }
                0x4 => return channel.gp_a_bank_h_table_bank = value,
                0x5 => {
                    return channel.gp_byte_counter_h_indirect_addr =
                        (channel.gp_byte_counter_h_indirect_addr & 0xFF00) | value as u16
                }
                0x6 => {
                    return channel.gp_byte_counter_h_indirect_addr =
                        (channel.gp_byte_counter_h_indirect_addr & 0x00FF) | (value as u16) << 8
                }
                0x7 => return channel.h_indirect_bank = value,
                0x8 => {
                    return channel.h_cur_table_addr =
                        (channel.h_cur_table_addr & 0xFF00) | value as u16
                }
                0x9 => {
                    return channel.h_cur_table_addr =
                        (channel.h_cur_table_addr & 0xFF) | (value as u16) << 8
                }
                0xA => return channel.set_h_line_counter(value),
                0xB | 0xF => return channel.unused = value,
                _ => {}
            }
        }
        _ => {}
    }

    #[cfg(feature = "log")]
    if A::LOG {
        slog::warn!(
            emu.cpu.logger,
            "Unknown bus A IO {} write @ {:#08X}: {:#04X} @ {:#08X}",
            A::NAME,
            addr,
            value,
            emu.cpu.regs.code_bank_base() | emu.cpu.regs.pc as u32,
        );
    }
}

pub fn read_b_io<A: AccessType>(emu: &mut Emu, addr: u8) -> u8 {
    match addr {
        0x04..=0x06 | 0x08..=0x0A | 0x14..=0x16 | 0x18..=0x1A | 0x24..=0x26 | 0x28..=0x2A => {
            #[cfg(feature = "log")]
            if A::LOG {
                slog::warn!(
                    emu.cpu.logger,
                    "Write-only PPU1 register {} read @ 0x0021{:02X}",
                    A::NAME,
                    addr
                );
            }
            return emu.ppu.ppu1_mdr();
        }
        0x39 => return emu.ppu.read_vram_low::<A>(),
        0x3A => return emu.ppu.read_vram_high::<A>(),
        0x3B => return emu.ppu.read_palette::<A>(),
        0x3E => return emu.ppu.status77().0,
        0x3F => return emu.ppu.status78().0,
        0x40..=0x43 => return rand::random(),
        _ => {}
    }

    #[cfg(feature = "log")]
    if A::LOG {
        slog::warn!(
            emu.cpu.logger,
            "Unknown bus B IO {} read @ 0x0021{:02X} @ {:#08X}",
            A::NAME,
            addr,
            emu.cpu.regs.code_bank_base() | emu.cpu.regs.pc as u32,
        );
    }
    // TODO: Open bus
    0
}

#[allow(clippy::needless_return)] // With logging disabled, the returns are detected as needless
pub fn write_b_io<A: AccessType>(emu: &mut Emu, addr: u8, value: u8) {
    match addr {
        0x00 => return emu.ppu.set_display_control_0(ppu::DisplayControl0(value)),
        0x05 => return emu.ppu.set_bg_mode_control(ppu::BgModeControl(value)),
        0x07 => return emu.ppu.bgs[0].set_screen_control(ppu::BgScreenControl(value)),
        0x08 => return emu.ppu.bgs[1].set_screen_control(ppu::BgScreenControl(value)),
        0x09 => return emu.ppu.bgs[2].set_screen_control(ppu::BgScreenControl(value)),
        0x0A => return emu.ppu.bgs[3].set_screen_control(ppu::BgScreenControl(value)),
        0x0B => return emu.ppu.set_bg_char_control_12(ppu::BgCharControl(value)),
        0x0C => return emu.ppu.set_bg_char_control_34(ppu::BgCharControl(value)),
        0x0F => return emu.ppu.write_bg_x_scroll(ppu::BgIndex::new(1), value),
        0x10 => return emu.ppu.write_bg_y_scroll(ppu::BgIndex::new(1), value),
        0x11 => return emu.ppu.write_bg_x_scroll(ppu::BgIndex::new(2), value),
        0x12 => return emu.ppu.write_bg_y_scroll(ppu::BgIndex::new(2), value),
        0x13 => return emu.ppu.write_bg_x_scroll(ppu::BgIndex::new(3), value),
        0x14 => return emu.ppu.write_bg_y_scroll(ppu::BgIndex::new(3), value),
        0x15 => {
            return emu
                .ppu
                .vram
                .set_increment_control(ppu::vram::IncrementControl(value))
        }
        0x16 => return emu.ppu.vram.set_addr_low(value),
        0x17 => return emu.ppu.vram.set_addr_high(value),
        0x18 => return emu.ppu.write_vram_low(value),
        0x19 => return emu.ppu.write_vram_high(value),
        0x21 => return emu.ppu.palette.set_word_addr(value),
        0x22 => return emu.ppu.write_palette(value),
        0x2C => return emu.ppu.enabled_main_screen_layers = value,
        0x2D => return emu.ppu.enabled_sub_screen_layers = value,
        0x33 => return emu.ppu.set_display_control_1(ppu::DisplayControl1(value)),
        0x40..=0x43 => {}
        _ => {}
    }

    #[cfg(feature = "log")]
    if A::LOG {
        slog::warn!(
            emu.cpu.logger,
            "Unknown bus B IO {} write @ 0x0021{:02X}: {:#04X} @ {:#08X}",
            A::NAME,
            addr,
            value,
            emu.cpu.regs.code_bank_base() | emu.cpu.regs.pc as u32,
        );
    }
}

pub fn read<A: AccessType>(emu: &mut Emu, addr: u32) -> u8 {
    let bank = (addr >> 16) as u8;
    match bank {
        // System area
        0x00..=0x3F | 0x80..=0xBF => match (addr >> 8) as u8 {
            // WRAM system area mirror
            0x00..=0x1F => return emu.wram[addr as usize & 0x1FFF],

            // Bus B I/O
            0x21 if !A::IS_DMA => return read_b_io::<A>(emu, addr as u8),

            // Internal CPU registers (TODO: some of them might be visible to DMA?)
            0x40..=0x43 if !A::IS_DMA => return read_a_io::<A>(emu, addr),

            // LoROM and other free areas used by carts
            _ => {}
        },

        // WRAM
        0x7E..=0x7F => return emu.wram[addr as usize & 0x1_FFFF],

        // HiROM
        _ => {}
    }

    if let Some(result) = emu.cart.read_data(addr) {
        return result;
    }

    #[cfg(feature = "log")]
    if A::LOG {
        slog::warn!(
            emu.cpu.logger,
            "Unknown bus A {} read @ {:#08X} @ {:#08X}",
            A::NAME,
            addr,
            emu.cpu.regs.code_bank_base() | emu.cpu.regs.pc as u32,
        );
    }
    0xFF
}

#[allow(clippy::needless_return)] // With logging disabled, the return is detected as needless
pub fn write<A: AccessType>(emu: &mut Emu, addr: u32, value: u8) {
    let bank = (addr >> 16) as u8;
    match bank {
        // System area
        0x00..=0x3F | 0x80..=0xBF => match (addr >> 8) as u8 {
            // WRAM system area mirror
            0x00..=0x1F => return emu.wram[addr as usize & 0x1FFF] = value,

            // Bus B I/O
            0x21 if !A::IS_DMA => return write_b_io::<A>(emu, addr as u8, value),

            // Internal CPU registers (TODO: some of them might be visible to DMA?)
            0x40..=0x43 if !A::IS_DMA => return write_a_io::<A>(emu, addr, value),

            // LoROM and other free areas used by carts
            _ => {}
        },

        // WRAM
        0x7E..=0x7F => {
            emu.wram[addr as usize & 0x1_FFFF] = value;
            return;
        }

        // HiROM
        _ => {}
    }

    if emu.cart.write_data(addr, value).is_some() {
        return;
    }

    #[cfg(feature = "log")]
    if A::LOG {
        slog::warn!(
            emu.cpu.logger,
            "Unknown bus A {} write @ {:#08X}: {:#04X} @ {:#08X}",
            A::NAME,
            addr,
            value,
            emu.cpu.regs.code_bank_base() | emu.cpu.regs.pc as u32,
        );
    }
}
