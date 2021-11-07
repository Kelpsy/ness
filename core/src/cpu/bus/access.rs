use crate::emu::Emu;

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
    0xFF
}

fn write_a_io<A: AccessType>(emu: &mut Emu, addr: u32, value: u8) {
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
    0xFF
}

#[allow(clippy::needless_return)] // With logging disabled, the returns are detected as needless
pub fn write_b_io<A: AccessType>(emu: &mut Emu, addr: u8, value: u8) {
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
