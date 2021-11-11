use crate::{
    apu::{spc700::bus, Apu},
    schedule::Timestamp,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AddrMode {
    Immediate,
    X,
    Y,
    Direct,
    DirectX,
    DirectY,
    DirectXIndirect,
    DirectIndirectY,
    Absolute,
    AbsoluteX,
    AbsoluteY,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reg {
    A,
    X,
    Y,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemOrReg {
    Reg(Reg),
    Mem(AddrMode),
}

pub fn read_8(apu: &mut Apu, addr: u16) -> u8 {
    let result = bus::read::<bus::ApuAccess>(apu, addr);
    apu.spc700.cur_timestamp += 1;
    result
}

pub fn read_8_dummy(apu: &mut Apu, addr: u16) -> u8 {
    let result = bus::read::<bus::ApuDummyAccess>(apu, addr);
    apu.spc700.cur_timestamp += 1;
    result
}

pub fn write_8(apu: &mut Apu, addr: u16, value: u8) {
    bus::write::<bus::ApuAccess>(apu, addr, value);
    apu.spc700.cur_timestamp += 1;
}

pub fn read_16(apu: &mut Apu, addr: u16) -> u16 {
    read_8(apu, addr) as u16 | (read_8(apu, addr.wrapping_add(1)) as u16) << 8
}

pub fn read_16_direct(apu: &mut Apu, addr: u8) -> u16 {
    read_8(apu, addr as u16 | apu.spc700.regs.direct_page_base()) as u16
        | (read_8(
            apu,
            addr.wrapping_add(1) as u16 | apu.spc700.regs.direct_page_base(),
        ) as u16)
            << 8
}

pub fn read_16_direct_idle(apu: &mut Apu, addr: u8) -> u16 {
    let low = read_8(apu, addr as u16 | apu.spc700.regs.direct_page_base());
    add_io_cycles(apu, 1);
    low as u16
        | (read_8(
            apu,
            addr.wrapping_add(1) as u16 | apu.spc700.regs.direct_page_base(),
        ) as u16)
            << 8
}

pub fn write_16_direct(apu: &mut Apu, addr: u8, value: u16) {
    write_8(
        apu,
        addr as u16 | apu.spc700.regs.direct_page_base(),
        value as u8,
    );
    write_8(
        apu,
        addr.wrapping_add(1) as u16 | apu.spc700.regs.direct_page_base(),
        (value >> 8) as u8,
    );
}

pub fn set_nz_8(apu: &mut Apu, value: u8) {
    apu.spc700.regs.psw = apu
        .spc700
        .regs
        .psw
        .with_negative(value >> 7 != 0)
        .with_zero(value == 0);
}

pub fn set_nz_16(apu: &mut Apu, value: u16) {
    apu.spc700.regs.psw = apu
        .spc700
        .regs
        .psw
        .with_negative(value >> 15 != 0)
        .with_zero(value == 0);
}

pub fn add_io_cycles(apu: &mut Apu, cycles: u8) {
    apu.spc700.cur_timestamp += cycles as Timestamp;
}

pub fn consume_imm_8(apu: &mut Apu) -> u8 {
    let res = read_8(apu, apu.spc700.regs.pc);
    apu.spc700.regs.pc = apu.spc700.regs.pc.wrapping_add(1);
    res
}

pub fn consume_imm_16(apu: &mut Apu) -> u16 {
    consume_imm_8(apu) as u16 | (consume_imm_8(apu) as u16) << 8
}

pub fn push_8(apu: &mut Apu, value: u8) {
    let sp = apu.spc700.regs.sp;
    write_8(apu, sp as u16 | 0x100, value);
    apu.spc700.regs.sp = sp.wrapping_sub(1);
}

pub fn push_16(apu: &mut Apu, value: u16) {
    push_8(apu, (value >> 8) as u8);
    push_8(apu, value as u8);
}

pub fn pop_8(apu: &mut Apu) -> u8 {
    apu.spc700.regs.sp = apu.spc700.regs.sp.wrapping_add(1);
    read_8(apu, apu.spc700.regs.sp as u16 | 0x100)
}

pub fn pop_16(apu: &mut Apu) -> u16 {
    pop_8(apu) as u16 | (pop_8(apu) as u16) << 8
}

fn read_effective_addr<const ADDR: AddrMode>(apu: &mut Apu) -> u16 {
    match ADDR {
        AddrMode::Immediate => unreachable!(),
        AddrMode::X => apu.spc700.regs.x as u16 | apu.spc700.regs.direct_page_base(),
        AddrMode::Y => apu.spc700.regs.y as u16 | apu.spc700.regs.direct_page_base(),
        AddrMode::Direct => consume_imm_8(apu) as u16 | apu.spc700.regs.direct_page_base(),
        AddrMode::DirectX => {
            let base = consume_imm_8(apu);
            add_io_cycles(apu, 1);
            base.wrapping_add(apu.spc700.regs.x) as u16 | apu.spc700.regs.direct_page_base()
        }
        AddrMode::DirectY => {
            let base = consume_imm_8(apu);
            add_io_cycles(apu, 1);
            base.wrapping_add(apu.spc700.regs.y) as u16 | apu.spc700.regs.direct_page_base()
        }
        AddrMode::DirectXIndirect => {
            let indirect = consume_imm_8(apu).wrapping_add(apu.spc700.regs.x);
            add_io_cycles(apu, 1);
            read_16_direct(apu, indirect)
        }
        AddrMode::DirectIndirectY => {
            let indirect = consume_imm_8(apu);
            let addr = read_16_direct(apu, indirect).wrapping_add(apu.spc700.regs.y as u16);
            add_io_cycles(apu, 1);
            addr
        }
        AddrMode::Absolute => consume_imm_16(apu),
        AddrMode::AbsoluteX => {
            let addr = consume_imm_16(apu).wrapping_add(apu.spc700.regs.x as u16);
            add_io_cycles(apu, 1);
            addr
        }
        AddrMode::AbsoluteY => {
            let addr = consume_imm_16(apu).wrapping_add(apu.spc700.regs.y as u16);
            add_io_cycles(apu, 1);
            addr
        }
    }
}

fn read_mem_or_reg_effective_addr<const OP: MemOrReg>(apu: &mut Apu) -> u16 {
    match OP {
        MemOrReg::Reg(_) => unreachable!(),
        MemOrReg::Mem(addr) => match addr {
            AddrMode::Immediate => unreachable!(),
            AddrMode::Direct => read_effective_addr::<{ AddrMode::Direct }>(apu),
            AddrMode::DirectX => read_effective_addr::<{ AddrMode::DirectX }>(apu),
            AddrMode::DirectY => read_effective_addr::<{ AddrMode::DirectY }>(apu),
            AddrMode::X => read_effective_addr::<{ AddrMode::X }>(apu),
            AddrMode::Y => read_effective_addr::<{ AddrMode::Y }>(apu),
            AddrMode::DirectXIndirect => read_effective_addr::<{ AddrMode::DirectXIndirect }>(apu),
            AddrMode::DirectIndirectY => read_effective_addr::<{ AddrMode::DirectIndirectY }>(apu),
            AddrMode::Absolute => read_effective_addr::<{ AddrMode::Absolute }>(apu),
            AddrMode::AbsoluteX => read_effective_addr::<{ AddrMode::AbsoluteX }>(apu),
            AddrMode::AbsoluteY => read_effective_addr::<{ AddrMode::AbsoluteY }>(apu),
        },
    }
}

pub fn read_reg<const REG: Reg>(apu: &mut Apu) -> u8 {
    match REG {
        Reg::A => apu.spc700.regs.a,
        Reg::X => apu.spc700.regs.x,
        Reg::Y => apu.spc700.regs.y,
    }
}

pub fn write_reg<const REG: Reg>(apu: &mut Apu, value: u8) {
    match REG {
        Reg::A => apu.spc700.regs.a = value,
        Reg::X => apu.spc700.regs.x = value,
        Reg::Y => apu.spc700.regs.y = value,
    }
}

pub fn do_addr_mode_read<const ADDR_MODE: AddrMode>(apu: &mut Apu) -> u8 {
    match ADDR_MODE {
        AddrMode::Immediate => consume_imm_8(apu),
        _ => {
            let addr = read_effective_addr::<ADDR_MODE>(apu);
            if matches!(ADDR_MODE, AddrMode::X | AddrMode::Y) {
                read_8_dummy(apu, apu.spc700.regs.pc);
            }
            read_8(apu, addr)
        }
    }
}

pub fn do_mem_or_reg_read<const OP: MemOrReg>(apu: &mut Apu) -> u8 {
    match OP {
        MemOrReg::Reg(reg) => match reg {
            Reg::A => apu.spc700.regs.a,
            Reg::X => apu.spc700.regs.x,
            Reg::Y => apu.spc700.regs.y,
        },
        MemOrReg::Mem(addr_mode) => match addr_mode {
            AddrMode::Immediate => consume_imm_8(apu),
            _ => {
                let addr = read_mem_or_reg_effective_addr::<OP>(apu);
                read_8(apu, addr)
            }
        },
    }
}

pub fn do_mem_or_reg_rmw<F: FnOnce(&mut Apu, u8) -> u8, const OP: MemOrReg>(apu: &mut Apu, f: F) {
    match OP {
        MemOrReg::Reg(reg) => {
            let value = do_mem_or_reg_read::<OP>(apu);
            let result = f(apu, value);
            match reg {
                Reg::A => apu.spc700.regs.a = result,
                Reg::X => apu.spc700.regs.x = result,
                Reg::Y => apu.spc700.regs.y = result,
            }
        }
        MemOrReg::Mem(_) => {
            let addr = read_mem_or_reg_effective_addr::<OP>(apu);
            let value = read_8(apu, addr);
            let result = f(apu, value);
            write_8(apu, addr, result);
        }
    }
}

pub fn do_mem_dummy_read_rmw<F: FnOnce(&mut Apu) -> u8, const ADDR_MODE: AddrMode>(
    apu: &mut Apu,
    f: F,
) {
    let addr = read_effective_addr::<ADDR_MODE>(apu);
    read_8_dummy(apu, addr);
    let result = f(apu);
    write_8(apu, addr, result);
}

pub fn do_mem_or_reg_dummy_write_rmw<F: FnOnce(&mut Apu, u8), const OP: MemOrReg>(
    apu: &mut Apu,
    f: F,
) {
    match OP {
        MemOrReg::Reg(_) => {
            let value = do_mem_or_reg_read::<OP>(apu);
            f(apu, value);
        }
        MemOrReg::Mem(_) => {
            let addr = read_mem_or_reg_effective_addr::<OP>(apu);
            let value = read_8(apu, addr);
            f(apu, value);
            add_io_cycles(apu, 1);
        }
    }
}
