use super::common::{
    add_io_cycles, consume_imm_16, consume_imm_8, pop_16, pop_8, push_16, push_8, read_16, read_8,
    read_8_dummy, set_nz_8, write_8,
};
use crate::apu::{spc700::regs::Psw, Apu};

pub fn nop(apu: &mut Apu) {
    read_8_dummy(apu, apu.spc700.regs.pc);
}

pub fn set_direct_page<const VALUE: bool>(apu: &mut Apu) {
    read_8_dummy(apu, apu.spc700.regs.pc);
    apu.spc700
        .regs
        .set_psw(apu.spc700.regs.psw().with_direct_page(VALUE));
}

pub fn set_carry<const VALUE: bool>(apu: &mut Apu) {
    read_8_dummy(apu, apu.spc700.regs.pc);
    apu.spc700.regs.psw.set_carry(VALUE);
}

pub fn notc(apu: &mut Apu) {
    apu.spc700.regs.psw.set_carry(!apu.spc700.regs.psw.carry());
    add_io_cycles(apu, 2);
}

pub fn clrv(apu: &mut Apu) {
    read_8_dummy(apu, apu.spc700.regs.pc);
    apu.spc700.regs.psw.set_overflow(false);
    apu.spc700.regs.psw.set_half_carry(false);
}

pub fn or1<const INVERT: bool>(apu: &mut Apu) {
    let addr_bit = consume_imm_16(apu);
    let value = read_8(apu, addr_bit & 0x1FFF);
    let mut bit_value = value & 1 << (addr_bit >> 13) != 0;
    if INVERT {
        bit_value = !bit_value;
    }
    apu.spc700
        .regs
        .psw
        .set_carry(apu.spc700.regs.psw.carry() | bit_value);
    add_io_cycles(apu, 1);
}

pub fn and1<const INVERT: bool>(apu: &mut Apu) {
    let addr_bit = consume_imm_16(apu);
    let value = read_8(apu, addr_bit & 0x1FFF);
    let mut bit_value = value & 1 << (addr_bit >> 13) != 0;
    if INVERT {
        bit_value = !bit_value;
    }
    apu.spc700
        .regs
        .psw
        .set_carry(apu.spc700.regs.psw.carry() & bit_value);
}

pub fn eor1(apu: &mut Apu) {
    let addr_bit = consume_imm_16(apu);
    let value = read_8(apu, addr_bit & 0x1FFF);
    let bit_value = value & 1 << (addr_bit >> 13) != 0;
    apu.spc700
        .regs
        .psw
        .set_carry(apu.spc700.regs.psw.carry() ^ bit_value);
    add_io_cycles(apu, 1);
}

pub fn mov1_c_mem(apu: &mut Apu) {
    let addr_bit = consume_imm_16(apu);
    let value = read_8(apu, addr_bit & 0x1FFF);
    let bit_value = value & 1 << (addr_bit >> 13) != 0;
    apu.spc700.regs.psw.set_carry(bit_value);
}

pub fn mov1_mem_c(apu: &mut Apu) {
    let addr_bit = consume_imm_16(apu);
    let addr = addr_bit & 0x1FFF;
    let bit = addr_bit >> 13;
    let value = read_8(apu, addr);
    add_io_cycles(apu, 1);
    write_8(
        apu,
        addr,
        (value & !(1 << bit)) | (apu.spc700.regs.psw.carry() as u8) << bit,
    );
}

pub fn not1(apu: &mut Apu) {
    let addr_bit = consume_imm_16(apu);
    let addr = addr_bit & 0x1FFF;
    let value = read_8(apu, addr);
    write_8(apu, addr, value ^ (1 << (addr_bit >> 13)));
}

pub fn modify_bit<const SET: bool, const BIT: u8>(apu: &mut Apu) {
    let addr = consume_imm_8(apu) as u16 | apu.spc700.regs.direct_page_base();
    let value = read_8(apu, addr);
    write_8(
        apu,
        addr,
        if SET {
            value | 1 << BIT
        } else {
            value & !(1 << BIT)
        },
    );
}

pub fn test_modify_bit<const SET: bool>(apu: &mut Apu) {
    let addr = consume_imm_16(apu);
    let value = read_8(apu, addr);
    set_nz_8(apu, apu.spc700.regs.a.wrapping_sub(value));
    write_8(
        apu,
        addr,
        if SET {
            value | apu.spc700.regs.a
        } else {
            value & !apu.spc700.regs.a
        },
    );
    add_io_cycles(apu, 1);
}

pub fn set_irqs_enabled<const VALUE: bool>(apu: &mut Apu) {
    read_8_dummy(apu, apu.spc700.regs.pc);
    apu.spc700.regs.psw.set_irqs_enabled(VALUE);
    add_io_cycles(apu, 1);
}

pub fn brk(apu: &mut Apu) {
    read_8_dummy(apu, apu.spc700.regs.pc);
    push_16(apu, apu.spc700.regs.pc);
    push_8(apu, apu.spc700.regs.psw.0);
    apu.spc700.regs.pc = read_16(apu, 0xFFDE);
    apu.spc700.regs.psw.set_irqs_enabled(false);
    apu.spc700.regs.psw.set_break_flag(true);
    add_io_cycles(apu, 1);
}

pub fn reti(apu: &mut Apu) {
    read_8_dummy(apu, apu.spc700.regs.pc);
    add_io_cycles(apu, 1);
    let new_psw = Psw(pop_8(apu));
    apu.spc700.regs.set_psw(new_psw);
    apu.spc700.regs.pc = pop_16(apu);
}

pub fn sleep(_apu: &mut Apu) {
    todo!("sleep");
}

pub fn stop(_apu: &mut Apu) {
    todo!("stop");
}
