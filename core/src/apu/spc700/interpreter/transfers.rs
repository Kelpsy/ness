use super::common::{
    add_io_cycles, consume_imm_8, do_mem_dummy_read_rmw, do_mem_or_reg_read, pop_8, push_8,
    read_16_direct_idle, read_8, read_8_dummy, read_reg, set_nz_16, set_nz_8, write_16_direct,
    write_8, write_reg, AddrMode, MemOrReg, Reg,
};
use crate::apu::{spc700::regs::Psw, Apu};

pub fn mov_x_sp(apu: &mut Apu) {
    read_8_dummy(apu, apu.spc700.regs.pc);
    apu.spc700.regs.x = apu.spc700.regs.sp;
    set_nz_8(apu, apu.spc700.regs.x);
}

pub fn mov_sp_x(apu: &mut Apu) {
    read_8_dummy(apu, apu.spc700.regs.pc);
    apu.spc700.regs.sp = apu.spc700.regs.x;
}

pub fn mov_reg_op<const REG: Reg, const SRC: MemOrReg>(apu: &mut Apu) {
    if matches!(SRC, MemOrReg::Reg(_)) {
        read_8_dummy(apu, apu.spc700.regs.pc);
    }
    let value = do_mem_or_reg_read::<SRC>(apu);
    write_reg::<REG>(apu, value);
    set_nz_8(apu, value);
}

pub fn mov_mem_reg<const DEST: AddrMode, const REG: Reg>(apu: &mut Apu) {
    let value = read_reg::<REG>(apu);
    do_mem_dummy_read_rmw::<_, DEST>(apu, |_| value);
}

pub fn mov_direct_imm(apu: &mut Apu) {
    let value = consume_imm_8(apu);
    let addr = consume_imm_8(apu) as u16 | apu.spc700.regs.direct_page_base();
    read_8_dummy(apu, addr);
    write_8(apu, addr, value);
}

pub fn mov_direct(apu: &mut Apu) {
    let src_addr = consume_imm_8(apu) as u16 | apu.spc700.regs.direct_page_base();
    let result = read_8(apu, src_addr);
    let dst_addr = consume_imm_8(apu) as u16 | apu.spc700.regs.direct_page_base();
    write_8(apu, dst_addr, result);
}

pub fn mov_a_mem_x_inc(apu: &mut Apu) {
    read_8_dummy(apu, apu.spc700.regs.pc);
    let addr = apu.spc700.regs.x as u16 | apu.spc700.regs.direct_page_base();
    apu.spc700.regs.x = apu.spc700.regs.x.wrapping_add(1);
    let result = read_8(apu, addr);
    apu.spc700.regs.a = result;
    set_nz_8(apu, result);
    add_io_cycles(apu, 1);
}

pub fn mov_mem_x_inc_a(apu: &mut Apu) {
    read_8_dummy(apu, apu.spc700.regs.pc);
    add_io_cycles(apu, 1);
    let addr = apu.spc700.regs.x as u16 | apu.spc700.regs.direct_page_base();
    apu.spc700.regs.x = apu.spc700.regs.x.wrapping_add(1);
    write_8(apu, addr, apu.spc700.regs.a);
}

pub fn movw_ya_direct(apu: &mut Apu) {
    let addr = consume_imm_8(apu);
    let result = read_16_direct_idle(apu, addr);
    apu.spc700.regs.set_ya(result);
    set_nz_16(apu, result);
}

pub fn movw_direct_ya(apu: &mut Apu) {
    let addr = consume_imm_8(apu);
    read_8_dummy(apu, addr as u16 | apu.spc700.regs.direct_page_base());
    let value = apu.spc700.regs.ya();
    write_16_direct(apu, addr, value);
}

pub fn push_reg<const REG: Reg>(apu: &mut Apu) {
    read_8_dummy(apu, apu.spc700.regs.pc);
    let value = read_reg::<REG>(apu);
    push_8(apu, value);
    add_io_cycles(apu, 1);
}

pub fn push_psw(apu: &mut Apu) {
    read_8_dummy(apu, apu.spc700.regs.pc);
    push_8(apu, apu.spc700.regs.psw.0);
    add_io_cycles(apu, 1);
}

pub fn pop_reg<const REG: Reg>(apu: &mut Apu) {
    read_8_dummy(apu, apu.spc700.regs.pc);
    add_io_cycles(apu, 1);
    let result = pop_8(apu);
    write_reg::<REG>(apu, result);
}

pub fn pop_psw(apu: &mut Apu) {
    read_8_dummy(apu, apu.spc700.regs.pc);
    add_io_cycles(apu, 1);
    let result = pop_8(apu);
    apu.spc700.regs.set_psw(Psw(result));
}
