use super::common::{
    add_io_cycles, do_addr_mode_read, do_addr_mode_write, do_rmw, set_nz, AddrMode, RegSize,
};
use crate::emu::Emu;

fn do_bin_adc<A: RegSize>(emu: &mut Emu, operand: A) {
    if A::IS_U16 {
        let src = emu.cpu.regs.a as u32;
        let operand = operand.as_zext_u16() as u32;
        let result = src + operand + emu.cpu.regs.psw.carry() as u32;
        emu.cpu.regs.psw.set_carry(result >> 16 != 0);
        emu.cpu
            .regs
            .psw
            .set_overflow(!(src ^ operand) & (src ^ result) & 1 << 15 != 0);
        let result = result as u16;
        set_nz(emu, result);
        emu.cpu.regs.a = result;
    } else {
        let src = emu.cpu.regs.a & 0xFF;
        let operand = operand.as_zext_u16();
        let result = src + operand + emu.cpu.regs.psw.carry() as u16;
        emu.cpu.regs.psw.set_carry(result >> 8 != 0);
        emu.cpu
            .regs
            .psw
            .set_overflow(!(src ^ operand) & (src ^ result) & 1 << 7 != 0);
        let result = result as u8;
        set_nz(emu, result);
        result.update_u16_low(&mut emu.cpu.regs.a);
    }
}

fn do_dec_adc<A: RegSize>(emu: &mut Emu, operand: A) {
    if A::IS_U16 {
        let src = emu.cpu.regs.a as u32;
        let operand = operand.as_zext_u16() as u32;
        let mut result = (src & 0xF) + (operand & 0xF) + emu.cpu.regs.psw.carry() as u32;
        if result > 9 {
            result += 6;
        }
        result = (src & 0xF0) + (operand & 0xF0) + (result & 0xF) + (((result > 0xF) as u32) << 4);
        if result > 0x9F {
            result += 0x60;
        }
        result =
            (src & 0xF00) + (operand & 0xF00) + (result & 0xFF) + (((result > 0xFF) as u32) << 8);
        if result > 0x9FF {
            result += 0x600;
        }
        result = (src & 0xF000)
            + (operand & 0xF000)
            + (result & 0xFFF)
            + (((result > 0xFFF) as u32) << 12);
        emu.cpu
            .regs
            .psw
            .set_overflow(!(src ^ operand) & (src ^ result) & 1 << 15 != 0);
        if result > 0x9FFF {
            result += 0x6000;
        }
        emu.cpu.regs.psw.set_carry(result >> 16 != 0);
        let result = result as u16;
        set_nz(emu, result);
        emu.cpu.regs.a = result;
    } else {
        let src = emu.cpu.regs.a & 0xFF;
        let operand = operand.as_zext_u16();
        let mut result = (src & 0xF) + (operand & 0xF) + emu.cpu.regs.psw.carry() as u16;
        if result > 9 {
            result += 6;
        }
        result = (src & 0xF0) + (operand & 0xF0) + (result & 0xF) + (((result > 0xF) as u16) << 4);
        emu.cpu
            .regs
            .psw
            .set_overflow(!(src ^ operand) & (src ^ result) & 1 << 7 != 0);
        if result > 0x9F {
            result += 0x60;
        }
        emu.cpu.regs.psw.set_carry(result >> 8 != 0);
        let result = result as u8;
        set_nz(emu, result);
        result.update_u16_low(&mut emu.cpu.regs.a);
    }
}

fn do_dec_sbc<A: RegSize>(emu: &mut Emu, operand: A) {
    if A::IS_U16 {
        let src = emu.cpu.regs.a as i32;
        let operand = operand.as_zext_u16() as i32;
        let mut result = (src & 0xF) + (operand & 0xF) + emu.cpu.regs.psw.carry() as i32;
        if result <= 0xF {
            result -= 6;
        }
        result = (src & 0xF0) + (operand & 0xF0) + (result & 0xF) + (((result > 0xF) as i32) << 4);
        if result <= 0xFF {
            result -= 0x60;
        }
        result =
            (src & 0xF00) + (operand & 0xF00) + (result & 0xFF) + (((result > 0xFF) as i32) << 8);
        if result <= 0xFFF {
            result -= 0x600;
        }
        result = (src & 0xF000)
            + (operand & 0xF000)
            + (result & 0xFFF)
            + (((result > 0xFFF) as i32) << 12);
        emu.cpu
            .regs
            .psw
            .set_overflow(!(src ^ operand) & (src ^ result) & 1 << 15 != 0);
        if result <= 0xFFFF {
            result = result.wrapping_sub(0x6000);
        }
        emu.cpu.regs.psw.set_carry(result > 0xFFFF);
        let result = result as u16;
        set_nz(emu, result);
        emu.cpu.regs.a = result;
    } else {
        let src = emu.cpu.regs.a as i16 & 0xFF;
        let operand = operand.as_zext_u16() as i16;
        let mut result = (src & 0xF) + (operand & 0xF) + emu.cpu.regs.psw.carry() as i16;
        if result <= 0xF {
            result = result.wrapping_sub(6);
        }
        result = (src & 0xF0) + (operand & 0xF0) + (result & 0xF) + (((result > 0xF) as i16) << 4);
        emu.cpu
            .regs
            .psw
            .set_overflow(!(src ^ operand) & (src ^ result) & 1 << 7 != 0);
        if result <= 0xFF {
            result = result.wrapping_sub(0x60);
        }
        emu.cpu.regs.psw.set_carry(result > 0xFF);
        let result = result as u8;
        set_nz(emu, result);
        result.update_u16_low(&mut emu.cpu.regs.a);
    }
}

fn do_compare<I: RegSize, T: RegSize, const ADDR: AddrMode>(emu: &mut Emu, op_a: u16) {
    let op_a = T::trunc_u16(op_a);
    let op_b = do_addr_mode_read::<I, T, ADDR>(emu);
    emu.cpu.regs.psw.set_carry(op_a >= op_b);
    set_nz(emu, op_a.wrapping_sub(op_b));
}

fn do_inc<T: RegSize>(emu: &mut Emu, src: T) -> T {
    add_io_cycles(emu, 1);
    let result = src.wrapping_add(T::zext_u8(1));
    set_nz(emu, result);
    result
}

fn do_dec<T: RegSize>(emu: &mut Emu, src: T) -> T {
    add_io_cycles(emu, 1);
    let result = src.wrapping_sub(T::zext_u8(1));
    set_nz(emu, result);
    result
}

fn do_asl<T: RegSize>(emu: &mut Emu, src: T) -> T {
    add_io_cycles(emu, 1);
    if T::IS_U16 {
        let src = src.as_zext_u16();
        let result = src << 1;
        emu.cpu.regs.psw.set_carry(src >> 15 != 0);
        set_nz(emu, result);
        T::trunc_u16(result)
    } else {
        let src = src.as_trunc_u8();
        let result = src << 1;
        emu.cpu.regs.psw.set_carry(src >> 7 != 0);
        set_nz(emu, result);
        T::zext_u8(result)
    }
}

fn do_lsr<T: RegSize>(emu: &mut Emu, src: T) -> T {
    add_io_cycles(emu, 1);
    if T::IS_U16 {
        let src = src.as_zext_u16();
        let result = src >> 1;
        emu.cpu.regs.psw.set_carry(src & 1 != 0);
        set_nz(emu, result);
        T::trunc_u16(result)
    } else {
        let src = src.as_trunc_u8();
        let result = src >> 1;
        emu.cpu.regs.psw.set_carry(src & 1 != 0);
        set_nz(emu, result);
        T::zext_u8(result)
    }
}

fn do_rol<T: RegSize>(emu: &mut Emu, src: T) -> T {
    add_io_cycles(emu, 1);
    if T::IS_U16 {
        let src = src.as_zext_u16();
        let result = src << 1 | emu.cpu.regs.psw.carry() as u16;
        emu.cpu.regs.psw.set_carry(src >> 15 != 0);
        set_nz(emu, result);
        T::trunc_u16(result)
    } else {
        let src = src.as_trunc_u8();
        let result = src << 1 | emu.cpu.regs.psw.carry() as u8;
        emu.cpu.regs.psw.set_carry(src >> 7 != 0);
        set_nz(emu, result);
        T::zext_u8(result)
    }
}

fn do_ror<T: RegSize>(emu: &mut Emu, src: T) -> T {
    add_io_cycles(emu, 1);
    if T::IS_U16 {
        let src = src.as_zext_u16();
        let result = src >> 1 | (emu.cpu.regs.psw.carry() as u16) << 15;
        emu.cpu.regs.psw.set_carry(src & 1 != 0);
        set_nz(emu, result);
        T::trunc_u16(result)
    } else {
        let src = src.as_trunc_u8();
        let result = src >> 1 | (emu.cpu.regs.psw.carry() as u8) << 7;
        emu.cpu.regs.psw.set_carry(src & 1 != 0);
        set_nz(emu, result);
        T::zext_u8(result)
    }
}

pub fn lda<A: RegSize, I: RegSize, const ADDR: AddrMode>(emu: &mut Emu) {
    let result = do_addr_mode_read::<I, A, ADDR>(emu);
    result.update_u16_low(&mut emu.cpu.regs.a);
    set_nz(emu, result);
}

pub fn sta<A: RegSize, I: RegSize, const ADDR: AddrMode>(emu: &mut Emu) {
    do_addr_mode_write::<I, A, ADDR>(emu, A::trunc_u16(emu.cpu.regs.a));
}

pub fn ora<A: RegSize, I: RegSize, const ADDR: AddrMode>(emu: &mut Emu) {
    let operand = do_addr_mode_read::<I, A, ADDR>(emu);
    let result = A::trunc_u16(emu.cpu.regs.a) | operand;
    result.update_u16_low(&mut emu.cpu.regs.a);
    set_nz(emu, result);
}

pub fn and<A: RegSize, I: RegSize, const ADDR: AddrMode>(emu: &mut Emu) {
    let operand = do_addr_mode_read::<I, A, ADDR>(emu);
    let result = A::trunc_u16(emu.cpu.regs.a) & operand;
    result.update_u16_low(&mut emu.cpu.regs.a);
    set_nz(emu, result);
}

pub fn eor<A: RegSize, I: RegSize, const ADDR: AddrMode>(emu: &mut Emu) {
    let operand = do_addr_mode_read::<I, A, ADDR>(emu);
    let result = A::trunc_u16(emu.cpu.regs.a) ^ operand;
    result.update_u16_low(&mut emu.cpu.regs.a);
    set_nz(emu, result);
}

pub fn adc<A: RegSize, I: RegSize, const ADDR: AddrMode, const DECIMAL: bool>(emu: &mut Emu) {
    let operand = do_addr_mode_read::<I, A, ADDR>(emu);
    if DECIMAL {
        do_dec_adc(emu, operand);
    } else {
        do_bin_adc(emu, operand);
    }
}

pub fn sbc<A: RegSize, I: RegSize, const ADDR: AddrMode, const DECIMAL: bool>(emu: &mut Emu) {
    let operand = !do_addr_mode_read::<I, A, ADDR>(emu);
    if DECIMAL {
        do_dec_sbc(emu, operand);
    } else {
        do_bin_adc(emu, operand);
    }
}

pub fn cmp<A: RegSize, I: RegSize, const ADDR: AddrMode>(emu: &mut Emu) {
    do_compare::<I, A, ADDR>(emu, emu.cpu.regs.a);
}

pub fn inc_a<A: RegSize>(emu: &mut Emu) {
    do_inc(emu, A::trunc_u16(emu.cpu.regs.a)).update_u16_low(&mut emu.cpu.regs.a);
}

pub fn inc<A: RegSize, I: RegSize, const ADDR: AddrMode>(emu: &mut Emu) {
    do_rmw::<_, I, A, ADDR>(emu, do_inc);
}

pub fn dec_a<A: RegSize>(emu: &mut Emu) {
    do_dec(emu, A::trunc_u16(emu.cpu.regs.a)).update_u16_low(&mut emu.cpu.regs.a);
}

pub fn dec<A: RegSize, I: RegSize, const ADDR: AddrMode>(emu: &mut Emu) {
    do_rmw::<_, I, A, ADDR>(emu, do_dec);
}

pub fn asl_a<A: RegSize>(emu: &mut Emu) {
    do_asl(emu, A::trunc_u16(emu.cpu.regs.a)).update_u16_low(&mut emu.cpu.regs.a);
}

pub fn asl<A: RegSize, I: RegSize, const ADDR: AddrMode>(emu: &mut Emu) {
    do_rmw::<_, I, A, ADDR>(emu, do_asl);
}

pub fn lsr_a<A: RegSize>(emu: &mut Emu) {
    do_lsr(emu, A::trunc_u16(emu.cpu.regs.a)).update_u16_low(&mut emu.cpu.regs.a);
}

pub fn lsr<A: RegSize, I: RegSize, const ADDR: AddrMode>(emu: &mut Emu) {
    do_rmw::<_, I, A, ADDR>(emu, do_lsr);
}

pub fn rol_a<A: RegSize>(emu: &mut Emu) {
    do_rol(emu, A::trunc_u16(emu.cpu.regs.a)).update_u16_low(&mut emu.cpu.regs.a);
}

pub fn rol<A: RegSize, I: RegSize, const ADDR: AddrMode>(emu: &mut Emu) {
    do_rmw::<_, I, A, ADDR>(emu, do_rol);
}

pub fn ror_a<A: RegSize>(emu: &mut Emu) {
    do_ror(emu, A::trunc_u16(emu.cpu.regs.a)).update_u16_low(&mut emu.cpu.regs.a);
}

pub fn ror<A: RegSize, I: RegSize, const ADDR: AddrMode>(emu: &mut Emu) {
    do_rmw::<_, I, A, ADDR>(emu, do_ror);
}

pub fn bit<A: RegSize, I: RegSize, const ADDR: AddrMode>(emu: &mut Emu) {
    let operand = do_addr_mode_read::<I, A, ADDR>(emu);
    let result = A::trunc_u16(emu.cpu.regs.a) & operand;
    emu.cpu.regs.psw.set_zero(result.is_zero());
    if ADDR != AddrMode::Immediate {
        emu.cpu.regs.psw.0 = (emu.cpu.regs.psw.0 & !0xC0)
            | if A::IS_U16 {
                (operand.as_zext_u16() >> 8) as u8 & 0xC0
            } else {
                operand.as_trunc_u8() & 0xC0
            };
    }
}

pub fn tsb<A: RegSize, const ADDR: AddrMode>(emu: &mut Emu) {
    do_rmw::<_, u8, A, ADDR>(emu, |emu, value| {
        add_io_cycles(emu, 1);
        let a = A::trunc_u16(emu.cpu.regs.a);
        emu.cpu.regs.psw.set_zero((value & a).is_zero());
        value | a
    });
}

pub fn trb<A: RegSize, const ADDR: AddrMode>(emu: &mut Emu) {
    do_rmw::<_, u8, A, ADDR>(emu, |emu, value| {
        add_io_cycles(emu, 1);
        let a = A::trunc_u16(emu.cpu.regs.a);
        emu.cpu.regs.psw.set_zero((value & a).is_zero());
        value & !a
    });
}

pub fn cpx<I: RegSize, const ADDR: AddrMode>(emu: &mut Emu) {
    do_compare::<I, I, ADDR>(emu, emu.cpu.regs.x);
}

pub fn cpy<I: RegSize, const ADDR: AddrMode>(emu: &mut Emu) {
    do_compare::<I, I, ADDR>(emu, emu.cpu.regs.y);
}

pub fn inx<I: RegSize>(emu: &mut Emu) {
    emu.cpu.regs.x = do_inc(emu, I::trunc_u16(emu.cpu.regs.x)).as_zext_u16();
}

pub fn iny<I: RegSize>(emu: &mut Emu) {
    emu.cpu.regs.y = do_inc(emu, I::trunc_u16(emu.cpu.regs.y)).as_zext_u16();
}

pub fn dex<I: RegSize>(emu: &mut Emu) {
    emu.cpu.regs.x = do_dec(emu, I::trunc_u16(emu.cpu.regs.x)).as_zext_u16();
}

pub fn dey<I: RegSize>(emu: &mut Emu) {
    emu.cpu.regs.y = do_dec(emu, I::trunc_u16(emu.cpu.regs.y)).as_zext_u16();
}
