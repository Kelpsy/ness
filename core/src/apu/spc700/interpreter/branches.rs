use super::common::{
    add_io_cycles, consume_imm_16, consume_imm_8, pop_16, push_16, read_16, read_8, read_8_dummy,
    write_8,
};
use crate::apu::{spc700::regs::Psw, Apu};

fn do_cond_branch(apu: &mut Apu, cond: impl FnOnce(Psw) -> bool) {
    let offset = consume_imm_8(apu) as i8;
    if cond(apu.spc700.regs.psw) {
        add_io_cycles(apu, 2);
        apu.spc700.regs.pc = apu.spc700.regs.pc.wrapping_add(offset as u16);
    }
}

pub fn bra(apu: &mut Apu) {
    do_cond_branch(apu, |_| true);
}

pub fn b_cond<const BIT: u8, const SET: bool>(apu: &mut Apu) {
    do_cond_branch(apu, |psw| (psw.0 & 1 << BIT != 0) == SET);
}

pub fn jmp_absolute(apu: &mut Apu) {
    let new_pc = consume_imm_16(apu);
    apu.spc700.regs.pc = new_pc;
}

pub fn jmp_abs_x_indirect(apu: &mut Apu) {
    let indirect = consume_imm_16(apu).wrapping_add(apu.spc700.regs.x as u16);
    add_io_cycles(apu, 1);
    apu.spc700.regs.pc = read_16(apu, indirect);
}

pub fn cbne_direct(apu: &mut Apu) {
    let addr = consume_imm_8(apu) as u16 | apu.spc700.regs.direct_page_base();
    let value = read_8(apu, addr);
    add_io_cycles(apu, 1);
    let offset = consume_imm_8(apu) as i8;
    if apu.spc700.regs.a != value {
        add_io_cycles(apu, 2);
        apu.spc700.regs.pc = apu.spc700.regs.pc.wrapping_add(offset as u16);
    }
}

pub fn cbne_direct_x(apu: &mut Apu) {
    let addr = consume_imm_8(apu).wrapping_add(apu.spc700.regs.x) as u16
        | apu.spc700.regs.direct_page_base();
    add_io_cycles(apu, 1);
    let value = read_8(apu, addr);
    add_io_cycles(apu, 1);
    let offset = consume_imm_8(apu) as i8;
    if apu.spc700.regs.a != value {
        add_io_cycles(apu, 2);
        apu.spc700.regs.pc = apu.spc700.regs.pc.wrapping_add(offset as u16);
    }
}

pub fn dbnz_direct(apu: &mut Apu) {
    let addr = consume_imm_8(apu) as u16 | apu.spc700.regs.direct_page_base();
    let result = read_8(apu, addr).wrapping_sub(1);
    write_8(apu, addr, result);
    let offset = consume_imm_8(apu) as i8;
    if result != 0 {
        add_io_cycles(apu, 2);
        apu.spc700.regs.pc = apu.spc700.regs.pc.wrapping_add(offset as u16);
    }
}

pub fn dbnz_y(apu: &mut Apu) {
    read_8_dummy(apu, apu.spc700.regs.pc);
    add_io_cycles(apu, 1);
    let result = apu.spc700.regs.y.wrapping_sub(1);
    apu.spc700.regs.y = result;
    let offset = consume_imm_8(apu) as i8;
    if result != 0 {
        add_io_cycles(apu, 2);
        apu.spc700.regs.pc = apu.spc700.regs.pc.wrapping_add(offset as u16);
    }
}

pub fn bbs<const BIT: u8>(apu: &mut Apu) {
    let addr = consume_imm_8(apu) as u16 | apu.spc700.regs.direct_page_base();
    let value = read_8(apu, addr);
    add_io_cycles(apu, 1);
    let offset = consume_imm_8(apu) as i8;
    if value & 1 << BIT != 0 {
        add_io_cycles(apu, 2);
        apu.spc700.regs.pc = apu.spc700.regs.pc.wrapping_add(offset as u16);
    }
}

pub fn bbc<const BIT: u8>(apu: &mut Apu) {
    let addr = consume_imm_8(apu) as u16 | apu.spc700.regs.direct_page_base();
    let value = read_8(apu, addr);
    add_io_cycles(apu, 1);
    let offset = consume_imm_8(apu) as i8;
    if value & 1 << BIT == 0 {
        add_io_cycles(apu, 2);
        apu.spc700.regs.pc = apu.spc700.regs.pc.wrapping_add(offset as u16);
    }
}

pub fn call(apu: &mut Apu) {
    let new_pc = consume_imm_16(apu);
    add_io_cycles(apu, 1);
    push_16(apu, apu.spc700.regs.pc);
    apu.spc700.regs.pc = new_pc;
    add_io_cycles(apu, 2);
}

pub fn pcall(apu: &mut Apu) {
    let new_pc = 0xFF00 | consume_imm_8(apu) as u16;
    add_io_cycles(apu, 1);
    push_16(apu, apu.spc700.regs.pc);
    apu.spc700.regs.pc = new_pc;
    add_io_cycles(apu, 1);
}

pub fn tcall<const INDEX: u8>(apu: &mut Apu) {
    read_8_dummy(apu, apu.spc700.regs.pc);
    add_io_cycles(apu, 1);
    push_16(apu, apu.spc700.regs.pc);
    add_io_cycles(apu, 1);
    let indirect_addr = 0xFFDE - 2 * INDEX as u16;
    apu.spc700.regs.pc = read_16(apu, indirect_addr);
}

pub fn ret(apu: &mut Apu) {
    read_8_dummy(apu, apu.spc700.regs.pc);
    add_io_cycles(apu, 1);
    apu.spc700.regs.pc = pop_16(apu);
}
