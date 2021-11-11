use super::common::{
    add_io_cycles, consume_imm_8, do_addr_mode_read, do_mem_or_reg_dummy_write_rmw,
    do_mem_or_reg_rmw, read_16_direct, read_16_direct_idle, read_8, read_8_dummy, set_nz_16,
    set_nz_8, write_8, AddrMode, MemOrReg,
};
use crate::apu::Apu;

fn do_adc(apu: &mut Apu, a: u8, b: u8) -> u8 {
    let a = a as u16;
    let b = b as u16;
    let carry_in = apu.spc700.regs.psw.carry() as u16;
    let result = a + b + carry_in;
    apu.spc700
        .regs
        .psw
        .set_half_carry(((a & 0xF) + (b & 0xF) + carry_in) >> 4 != 0);
    apu.spc700.regs.psw.set_carry(result >> 8 != 0);
    apu.spc700
        .regs
        .psw
        .set_overflow(!(a ^ b) & (a ^ result) & 1 << 7 != 0);
    let result = result as u8;
    set_nz_8(apu, result);
    result
}

fn rmw_16_direct_interleaved(apu: &mut Apu, addr: u8, f: impl FnOnce(u16) -> u16) -> u16 {
    let addr_low = addr as u16 | apu.spc700.regs.direct_page_base();
    let addr_high = (addr.wrapping_add(1)) as u16 | apu.spc700.regs.direct_page_base();
    let mut result = f(read_8(apu, addr_low) as u16);
    write_8(apu, addr_low, result as u8);
    result = result.wrapping_add((read_8(apu, addr_high) as u16) << 8);
    write_8(apu, addr_high, (result >> 8) as u8);
    result
}

pub fn or<const FIRST_OP: MemOrReg, const SECOND_OP: AddrMode>(apu: &mut Apu) {
    let b = do_addr_mode_read::<SECOND_OP>(apu);
    do_mem_or_reg_rmw::<_, FIRST_OP>(apu, |apu, a| {
        let result = a | b;
        set_nz_8(apu, result);
        result
    });
}

pub fn and<const FIRST_OP: MemOrReg, const SECOND_OP: AddrMode>(apu: &mut Apu) {
    let b = do_addr_mode_read::<SECOND_OP>(apu);
    do_mem_or_reg_rmw::<_, FIRST_OP>(apu, |apu, a| {
        let result = a & b;
        set_nz_8(apu, result);
        result
    });
}

pub fn eor<const FIRST_OP: MemOrReg, const SECOND_OP: AddrMode>(apu: &mut Apu) {
    let b = do_addr_mode_read::<SECOND_OP>(apu);
    do_mem_or_reg_rmw::<_, FIRST_OP>(apu, |apu, a| {
        let result = a ^ b;
        set_nz_8(apu, result);
        result
    });
}

pub fn adc<const FIRST_OP: MemOrReg, const SECOND_OP: AddrMode>(apu: &mut Apu) {
    let b = do_addr_mode_read::<SECOND_OP>(apu);
    do_mem_or_reg_rmw::<_, FIRST_OP>(apu, |apu, a| do_adc(apu, a, b));
}

pub fn sbc<const FIRST_OP: MemOrReg, const SECOND_OP: AddrMode>(apu: &mut Apu) {
    let b = do_addr_mode_read::<SECOND_OP>(apu);
    do_mem_or_reg_rmw::<_, FIRST_OP>(apu, |apu, a| do_adc(apu, a, !b));
}

pub fn cmp<const FIRST_OP: MemOrReg, const SECOND_OP: AddrMode>(apu: &mut Apu) {
    let b = do_addr_mode_read::<SECOND_OP>(apu);
    do_mem_or_reg_dummy_write_rmw::<_, FIRST_OP>(apu, |apu, a| {
        apu.spc700.regs.psw.set_carry(a >= b);
        set_nz_8(apu, a.wrapping_sub(b));
    });
}

pub fn addw(apu: &mut Apu) {
    let addr = consume_imm_8(apu);
    let a = apu.spc700.regs.ya() as u32;
    let b = read_16_direct_idle(apu, addr) as u32;
    let result = a + b;
    apu.spc700
        .regs
        .psw
        .set_half_carry(((a & 0xFFF) + (b & 0xFFF)) >> 12 != 0);
    apu.spc700.regs.psw.set_carry(result >> 16 != 0);
    apu.spc700
        .regs
        .psw
        .set_overflow(!(a ^ b) & (a ^ result) & 1 << 15 != 0);
    let result = result as u16;
    set_nz_16(apu, result);
    apu.spc700.regs.a = result as u8;
    apu.spc700.regs.y = (result >> 8) as u8;
}

pub fn subw(apu: &mut Apu) {
    let addr = consume_imm_8(apu);
    let a = apu.spc700.regs.ya() as u32;
    let b = read_16_direct_idle(apu, addr) as u32;
    let result = a.wrapping_sub(b);
    apu.spc700
        .regs
        .psw
        .set_half_carry((a & 0xFFF) >= (b & 0xFFF));
    apu.spc700.regs.psw.set_carry(a >= b);
    apu.spc700
        .regs
        .psw
        .set_overflow((a ^ b) & (a ^ result) & 1 << 15 != 0);
    let result = result as u16;
    set_nz_16(apu, result);
    apu.spc700.regs.a = result as u8;
    apu.spc700.regs.y = (result >> 8) as u8;
}

pub fn cmpw(apu: &mut Apu) {
    let addr = consume_imm_8(apu);
    let a = apu.spc700.regs.ya();
    let b = read_16_direct(apu, addr);
    apu.spc700.regs.psw.set_carry(a >= b);
    set_nz_16(apu, a.wrapping_sub(b));
}

pub fn asl<const OP: MemOrReg>(apu: &mut Apu) {
    do_mem_or_reg_rmw::<_, OP>(apu, |apu, value| {
        let result = value << 1;
        apu.spc700.regs.psw.set_carry(value >> 7 != 0);
        set_nz_8(apu, result);
        result
    });
    if matches!(OP, MemOrReg::Reg(_)) {
        add_io_cycles(apu, 1);
    }
}

pub fn lsr<const OP: MemOrReg>(apu: &mut Apu) {
    do_mem_or_reg_rmw::<_, OP>(apu, |apu, value| {
        let result = value >> 1;
        apu.spc700.regs.psw.set_carry(value & 1 != 0);
        set_nz_8(apu, result);
        result
    });
    if matches!(OP, MemOrReg::Reg(_)) {
        add_io_cycles(apu, 1);
    }
}

pub fn rol<const OP: MemOrReg>(apu: &mut Apu) {
    do_mem_or_reg_rmw::<_, OP>(apu, |apu, value| {
        let result = value << 1 | apu.spc700.regs.psw.carry() as u8;
        apu.spc700.regs.psw.set_carry(value >> 7 != 0);
        set_nz_8(apu, result);
        result
    });
    if matches!(OP, MemOrReg::Reg(_)) {
        add_io_cycles(apu, 1);
    }
}

pub fn ror<const OP: MemOrReg>(apu: &mut Apu) {
    do_mem_or_reg_rmw::<_, OP>(apu, |apu, value| {
        let result = value >> 1 | (apu.spc700.regs.psw.carry() as u8) << 7;
        apu.spc700.regs.psw.set_carry(value & 1 != 0);
        set_nz_8(apu, result);
        result
    });
    if matches!(OP, MemOrReg::Reg(_)) {
        add_io_cycles(apu, 1);
    }
}

pub fn inc<const OP: MemOrReg>(apu: &mut Apu) {
    do_mem_or_reg_rmw::<_, OP>(apu, |apu, value| {
        let result = value.wrapping_add(1);
        set_nz_8(apu, result);
        result
    });
    if matches!(OP, MemOrReg::Reg(_)) {
        add_io_cycles(apu, 1);
    }
}

pub fn dec<const OP: MemOrReg>(apu: &mut Apu) {
    do_mem_or_reg_rmw::<_, OP>(apu, |apu, value| {
        let result = value.wrapping_sub(1);
        set_nz_8(apu, result);
        result
    });
    if matches!(OP, MemOrReg::Reg(_)) {
        add_io_cycles(apu, 1);
    }
}

pub fn incw(apu: &mut Apu) {
    let addr = consume_imm_8(apu);
    let result = rmw_16_direct_interleaved(apu, addr, |v| v.wrapping_add(1));
    set_nz_16(apu, result);
}

pub fn decw(apu: &mut Apu) {
    let addr = consume_imm_8(apu);
    let result = rmw_16_direct_interleaved(apu, addr, |v| v.wrapping_sub(1));
    set_nz_16(apu, result);
}

pub fn mul(apu: &mut Apu) {
    read_8_dummy(apu, apu.spc700.regs.pc);
    let result = apu.spc700.regs.y as u16 * apu.spc700.regs.a as u16;
    apu.spc700.regs.set_ya(result);
    set_nz_8(apu, apu.spc700.regs.y);
    add_io_cycles(apu, 7);
}

pub fn div(apu: &mut Apu) {
    // Based on bsnes's algorithm:
    // https://github.com/bsnes-emu/bsnes/blob/master/bsnes/processor/spc700/instructions.cpp#L348
    read_8_dummy(apu, apu.spc700.regs.pc);
    apu.spc700
        .regs
        .psw
        .set_half_carry(apu.spc700.regs.y & 0xF >= apu.spc700.regs.x & 0xF);
    apu.spc700
        .regs
        .psw
        .set_overflow(apu.spc700.regs.y >= apu.spc700.regs.x);
    let ya = apu.spc700.regs.ya();
    let x = apu.spc700.regs.x as u16;
    if (apu.spc700.regs.y as u16) < x << 1 {
        apu.spc700.regs.a = (ya / x) as u8;
        apu.spc700.regs.y = (ya % x) as u8;
    } else {
        apu.spc700.regs.a = (255 - (ya - (x << 9)) / (256 - x)) as u8;
        apu.spc700.regs.y = (x + (ya - (x << 9)) % (256 - x)) as u8;
    };
    set_nz_8(apu, apu.spc700.regs.a);
    add_io_cycles(apu, 10);
}

pub fn xcn(apu: &mut Apu) {
    read_8_dummy(apu, apu.spc700.regs.pc);
    apu.spc700.regs.a = apu.spc700.regs.a.rotate_right(4);
    set_nz_8(apu, apu.spc700.regs.a);
    add_io_cycles(apu, 3);
}

pub fn daa(apu: &mut Apu) {
    read_8_dummy(apu, apu.spc700.regs.pc);
    if apu.spc700.regs.psw.carry() || apu.spc700.regs.a > 0x99 {
        apu.spc700.regs.a = apu.spc700.regs.a.wrapping_add(0x60);
        apu.spc700.regs.psw.set_carry(true);
    }
    if apu.spc700.regs.psw.half_carry() || apu.spc700.regs.a & 0xF > 9 {
        apu.spc700.regs.a = apu.spc700.regs.a.wrapping_add(6);
    }
    set_nz_8(apu, apu.spc700.regs.a);
    add_io_cycles(apu, 1);
}

pub fn das(apu: &mut Apu) {
    read_8_dummy(apu, apu.spc700.regs.pc);
    if !apu.spc700.regs.psw.carry() || apu.spc700.regs.a > 0x99 {
        apu.spc700.regs.a = apu.spc700.regs.a.wrapping_sub(0x60);
        apu.spc700.regs.psw.set_carry(false);
    }
    if !apu.spc700.regs.psw.half_carry() || apu.spc700.regs.a & 0xF > 9 {
        apu.spc700.regs.a = apu.spc700.regs.a.wrapping_sub(6);
    }
    set_nz_8(apu, apu.spc700.regs.a);
    add_io_cycles(apu, 1);
}
