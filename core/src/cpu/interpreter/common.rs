pub use super::super::common::{AddrMode, JumpAddr, RegSize};
use crate::{cpu::bus, emu::schedule::Timestamp, emu::Emu};

pub fn add_io_cycles(emu: &mut Emu, cycles: u8) {
    emu.cpu.cur_timestamp += cycles as Timestamp * 6;
}

pub fn read_8(emu: &mut Emu, addr: u32) -> u8 {
    let result = bus::read::<bus::CpuAccess>(emu, addr);
    emu.cpu.cur_timestamp += 6; // TODO: Use real timings
    result
}

pub fn write_8(emu: &mut Emu, addr: u32, value: u8) {
    bus::write::<bus::CpuAccess>(emu, addr, value);
    emu.cpu.cur_timestamp += 6; // TODO: Use real timings
}

pub fn read_16(emu: &mut Emu, addr: u32) -> u16 {
    read_8(emu, addr) as u16 | (read_8(emu, addr.wrapping_add(1)) as u16) << 8
}

pub fn read_16_bank0(emu: &mut Emu, addr: u16) -> u16 {
    read_8(emu, addr as u32) as u16 | (read_8(emu, addr.wrapping_add(1) as u32) as u16) << 8
}

pub fn write_16(emu: &mut Emu, addr: u32, value: u16) {
    write_8(emu, addr, value as u8);
    write_8(emu, addr.wrapping_add(1), (value >> 8) as u8);
}

pub fn write_16_bank0(emu: &mut Emu, addr: u16, value: u16) {
    write_8(emu, addr as u32, value as u8);
    write_8(emu, addr.wrapping_add(1) as u32, (value >> 8) as u8);
}

pub fn set_nz<T: RegSize>(emu: &mut Emu, value: T) {
    emu.cpu.regs.psw = emu
        .cpu
        .regs
        .psw
        .with_negative(value.is_negative())
        .with_zero(value.is_zero());
}

pub fn consume_imm<T: RegSize>(emu: &mut Emu) -> T {
    if T::IS_U16 {
        let code_bank_base = emu.cpu.regs.code_bank_base();
        let pc = emu.cpu.regs.pc;
        let res = read_8(emu, code_bank_base | pc as u32) as u16
            | (read_8(emu, code_bank_base | pc.wrapping_add(1) as u32) as u16) << 8;
        emu.cpu.regs.pc = pc.wrapping_add(2);
        T::trunc_u16(res)
    } else {
        let res = read_8(emu, emu.cpu.regs.code_bank_base() | emu.cpu.regs.pc as u32);
        emu.cpu.regs.pc = emu.cpu.regs.pc.wrapping_add(1);
        T::zext_u8(res)
    }
}

pub fn push<T: RegSize>(emu: &mut Emu, value: T) {
    let mut sp = emu.cpu.regs.sp;
    if T::IS_U16 {
        let value = value.as_zext_u16();
        write_8(emu, sp as u32, (value >> 8) as u8);
        sp = sp.wrapping_sub(1);
        write_8(emu, sp as u32, value as u8);
    } else {
        write_8(emu, emu.cpu.regs.sp as u32, value.as_trunc_u8());
    }
    emu.cpu.regs.sp = sp.wrapping_sub(1);
}

pub fn pull<T: RegSize>(emu: &mut Emu) -> T {
    if T::IS_U16 {
        let mut sp = emu.cpu.regs.sp.wrapping_add(1);
        let low = read_8(emu, sp as u32);
        sp = sp.wrapping_add(1);
        let high = read_8(emu, sp as u32);
        emu.cpu.regs.sp = sp;
        T::trunc_u16(low as u16 | (high as u16) << 8)
    } else {
        emu.cpu.regs.sp = emu.cpu.regs.sp.wrapping_add(1);
        T::zext_u8(read_8(emu, emu.cpu.regs.sp as u32))
    }
}

pub fn jump_to_exc_vector(emu: &mut Emu, addr: u16) {
    emu.cpu.regs.set_psw(
        emu.cpu
            .regs
            .psw
            .with_decimal_mode(false)
            .with_irqs_disabled(true),
    );
    // TODO: Update IRQs enabled
    emu.cpu.regs.pc = read_16_bank0(emu, addr);
    emu.cpu.regs.set_code_bank(0);
}

pub fn read_direct_addr(emu: &mut Emu) -> u16 {
    let dp_off = emu.cpu.regs.direct_page_offset;
    let result = dp_off.wrapping_add(consume_imm::<u8>(emu) as u16);
    if dp_off as u8 != 0 {
        add_io_cycles(emu, 1);
    }
    result
}

pub fn read_indirect_addr(emu: &mut Emu, addr: u16) -> u32 {
    read_16_bank0(emu, addr) as u32 | emu.cpu.regs.data_bank_base()
}

fn read_indirect_long_addr(emu: &mut Emu, addr: u16) -> u32 {
    read_16_bank0(emu, addr) as u32 | (read_8(emu, addr.wrapping_add(2) as u32) as u32) << 16
}

fn read_absolute_addr(emu: &mut Emu) -> u32 {
    consume_imm::<u16>(emu) as u32 | emu.cpu.regs.data_bank_base()
}

fn read_absolute_long_addr(emu: &mut Emu) -> u32 {
    consume_imm::<u16>(emu) as u32 | (consume_imm::<u8>(emu) as u32) << 16
}

fn read_stack_relative_addr(emu: &mut Emu) -> u16 {
    let addr = emu.cpu.regs.sp.wrapping_add(consume_imm::<u8>(emu) as u16);
    add_io_cycles(emu, 1);
    addr
}

fn add_index_32_io_cycles<I: RegSize, const WRITE: bool>(
    emu: &mut Emu,
    unindexed: u32,
    indexed: u32,
) {
    if I::IS_U16 || WRITE || unindexed >> 8 != indexed >> 8 {
        add_io_cycles(emu, 1);
    }
}

fn read_effective_addr<I: RegSize, const ADDR: AddrMode, const WRITE: bool>(emu: &mut Emu) -> u32 {
    match ADDR {
        AddrMode::Immediate => unreachable!(),
        AddrMode::Direct => read_direct_addr(emu) as u32,
        AddrMode::DirectX => {
            let unindexed = read_direct_addr(emu);
            add_io_cycles(emu, 1);
            unindexed.wrapping_add(emu.cpu.regs.x) as u32
        }
        AddrMode::DirectY => {
            let unindexed = read_direct_addr(emu);
            add_io_cycles(emu, 1);
            unindexed.wrapping_add(emu.cpu.regs.y) as u32
        }
        AddrMode::DirectIndirect => {
            let indirect = read_direct_addr(emu);
            read_indirect_addr(emu, indirect)
        }
        AddrMode::DirectXIndirect => {
            let indirect = read_direct_addr(emu).wrapping_add(emu.cpu.regs.x);
            add_io_cycles(emu, 1);
            read_indirect_addr(emu, indirect)
        }
        AddrMode::DirectIndirectY => {
            let indirect = read_direct_addr(emu);
            let unindexed = read_indirect_addr(emu, indirect);
            let addr = (unindexed + emu.cpu.regs.y as u32) & 0xFF_FFFF;
            add_index_32_io_cycles::<I, WRITE>(emu, unindexed, addr);
            addr
        }
        AddrMode::DirectIndirectLong => {
            let indirect = read_direct_addr(emu);
            read_indirect_long_addr(emu, indirect)
        }
        AddrMode::DirectIndirectLongY => {
            let indirect = read_direct_addr(emu);
            let unindexed = read_indirect_long_addr(emu, indirect);
            (unindexed + emu.cpu.regs.y as u32) & 0xFF_FFFF
        }
        AddrMode::Absolute => read_absolute_addr(emu),
        AddrMode::AbsoluteX => {
            let unindexed = read_absolute_addr(emu);
            let addr = (unindexed + emu.cpu.regs.x as u32) & 0xFF_FFFF;
            add_index_32_io_cycles::<I, WRITE>(emu, unindexed, addr);
            addr
        }
        AddrMode::AbsoluteY => {
            let unindexed = read_absolute_addr(emu);
            let addr = (unindexed + emu.cpu.regs.y as u32) & 0xFF_FFFF;
            add_index_32_io_cycles::<I, WRITE>(emu, unindexed, addr);
            addr
        }
        AddrMode::AbsoluteLong => read_absolute_long_addr(emu),
        AddrMode::AbsoluteLongX => {
            (read_absolute_long_addr(emu) + emu.cpu.regs.x as u32) & 0xFF_FFFF
        }
        AddrMode::StackRel => read_stack_relative_addr(emu) as u32,
        AddrMode::StackRelIndirectY => {
            let indirect = read_stack_relative_addr(emu);
            add_io_cycles(emu, 1);
            let unindexed = read_indirect_addr(emu, indirect);
            (unindexed + emu.cpu.regs.y as u32) & 0xFF_FFFF
        }
    }
}

pub fn do_addr_mode_read<I: RegSize, T: RegSize, const ADDR: AddrMode>(emu: &mut Emu) -> T {
    if ADDR == AddrMode::Immediate {
        consume_imm(emu)
    } else {
        let addr = read_effective_addr::<I, ADDR, false>(emu);
        if T::IS_U16 {
            T::trunc_u16(if ADDR.is_masked_to_direct_page() {
                read_16_bank0(emu, addr as u16)
            } else {
                read_16(emu, addr)
            })
        } else {
            T::zext_u8(read_8(emu, addr as u32))
        }
    }
}

pub fn do_addr_mode_write<I: RegSize, T: RegSize, const ADDR: AddrMode>(emu: &mut Emu, value: T) {
    let addr = read_effective_addr::<I, ADDR, true>(emu);
    if T::IS_U16 {
        if ADDR.is_masked_to_direct_page() {
            write_16_bank0(emu, addr as u16, value.as_zext_u16())
        } else {
            write_16(emu, addr, value.as_zext_u16())
        }
    } else {
        write_8(emu, addr as u32, value.as_trunc_u8());
    }
}

pub fn do_rmw<F: FnOnce(&mut Emu, T) -> T, I: RegSize, T: RegSize, const ADDR: AddrMode>(
    emu: &mut Emu,
    f: F,
) {
    let addr = read_effective_addr::<I, ADDR, true>(emu);
    if T::IS_U16 {
        if ADDR.is_masked_to_direct_page() {
            let value = read_16_bank0(emu, addr as u16);
            let result = f(emu, T::trunc_u16(value)).as_zext_u16();
            let addr = addr as u16;
            write_8(emu, addr.wrapping_add(1) as u32, (result >> 8) as u8);
            write_8(emu, addr as u32, result as u8);
        } else {
            let value = read_16(emu, addr);
            let result = f(emu, T::trunc_u16(value)).as_zext_u16();
            write_8(emu, addr.wrapping_add(1), (result >> 8) as u8);
            write_8(emu, addr, result as u8);
        }
    } else {
        let value = read_8(emu, addr as u32);
        let result = f(emu, T::zext_u8(value)).as_trunc_u8();
        write_8(emu, addr as u32, result);
    }
}
