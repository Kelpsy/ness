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

pub(super) fn lda<A: RegSize, I: RegSize, const ADDR: AddrMode>(emu: &mut Emu) {
    let result = do_addr_mode_read::<I, A, ADDR>(emu);
    result.update_u16_low(&mut emu.cpu.regs.a);
    set_nz(emu, result);
}

pub(super) fn sta<A: RegSize, I: RegSize, const ADDR: AddrMode>(emu: &mut Emu) {
    do_addr_mode_write::<I, A, ADDR>(emu, A::trunc_u16(emu.cpu.regs.a));
}

pub(super) fn ora<A: RegSize, I: RegSize, const ADDR: AddrMode>(emu: &mut Emu) {
    let operand = do_addr_mode_read::<I, A, ADDR>(emu);
    let result = A::trunc_u16(emu.cpu.regs.a) | operand;
    result.update_u16_low(&mut emu.cpu.regs.a);
    set_nz(emu, result);
}

pub(super) fn and<A: RegSize, I: RegSize, const ADDR: AddrMode>(emu: &mut Emu) {
    let operand = do_addr_mode_read::<I, A, ADDR>(emu);
    let result = A::trunc_u16(emu.cpu.regs.a) & operand;
    result.update_u16_low(&mut emu.cpu.regs.a);
    set_nz(emu, result);
}

pub(super) fn eor<A: RegSize, I: RegSize, const ADDR: AddrMode>(emu: &mut Emu) {
    let operand = do_addr_mode_read::<I, A, ADDR>(emu);
    let result = A::trunc_u16(emu.cpu.regs.a) ^ operand;
    result.update_u16_low(&mut emu.cpu.regs.a);
    set_nz(emu, result);
}

pub(super) fn adc<A: RegSize, I: RegSize, const ADDR: AddrMode, const DECIMAL: bool>(
    emu: &mut Emu,
) {
    let operand = do_addr_mode_read::<I, A, ADDR>(emu);
    if DECIMAL {
        todo!("Decimal ADC");
    } else {
        do_bin_adc(emu, operand);
    }
}

pub(super) fn sbc<A: RegSize, I: RegSize, const ADDR: AddrMode, const DECIMAL: bool>(
    emu: &mut Emu,
) {
    let operand = do_addr_mode_read::<I, A, ADDR>(emu);
    if DECIMAL {
        todo!("Decimal SBC");
    } else {
        do_bin_adc(emu, !operand);
    }
}

pub(super) fn cmp<A: RegSize, I: RegSize, const ADDR: AddrMode>(emu: &mut Emu) {
    do_compare::<I, A, ADDR>(emu, emu.cpu.regs.a);
}

pub(super) fn inc_a<A: RegSize>(emu: &mut Emu) {
    do_inc(emu, A::trunc_u16(emu.cpu.regs.a)).update_u16_low(&mut emu.cpu.regs.a);
}

pub(super) fn inc<A: RegSize, I: RegSize, const ADDR: AddrMode>(emu: &mut Emu) {
    do_rmw::<_, I, A, ADDR>(emu, do_inc);
}

pub(super) fn dec_a<A: RegSize>(emu: &mut Emu) {
    do_dec(emu, A::trunc_u16(emu.cpu.regs.a)).update_u16_low(&mut emu.cpu.regs.a);
}

pub(super) fn dec<A: RegSize, I: RegSize, const ADDR: AddrMode>(emu: &mut Emu) {
    do_rmw::<_, I, A, ADDR>(emu, do_dec);
}

pub(super) fn asl_a<A: RegSize>(emu: &mut Emu) {
    do_asl(emu, A::trunc_u16(emu.cpu.regs.a)).update_u16_low(&mut emu.cpu.regs.a);
}

pub(super) fn asl<A: RegSize, I: RegSize, const ADDR: AddrMode>(emu: &mut Emu) {
    do_rmw::<_, I, A, ADDR>(emu, do_asl);
}

pub(super) fn lsr_a<A: RegSize>(emu: &mut Emu) {
    do_lsr(emu, A::trunc_u16(emu.cpu.regs.a)).update_u16_low(&mut emu.cpu.regs.a);
}

pub(super) fn lsr<A: RegSize, I: RegSize, const ADDR: AddrMode>(emu: &mut Emu) {
    do_rmw::<_, I, A, ADDR>(emu, do_lsr);
}

pub(super) fn rol_a<A: RegSize>(emu: &mut Emu) {
    do_rol(emu, A::trunc_u16(emu.cpu.regs.a)).update_u16_low(&mut emu.cpu.regs.a);
}

pub(super) fn rol<A: RegSize, I: RegSize, const ADDR: AddrMode>(emu: &mut Emu) {
    do_rmw::<_, I, A, ADDR>(emu, do_rol);
}

pub(super) fn ror_a<A: RegSize>(emu: &mut Emu) {
    do_ror(emu, A::trunc_u16(emu.cpu.regs.a)).update_u16_low(&mut emu.cpu.regs.a);
}

pub(super) fn ror<A: RegSize, I: RegSize, const ADDR: AddrMode>(emu: &mut Emu) {
    do_rmw::<_, I, A, ADDR>(emu, do_ror);
}

pub(super) fn bit<A: RegSize, I: RegSize, const ADDR: AddrMode>(emu: &mut Emu) {
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

pub(super) fn tsb<A: RegSize, const ADDR: AddrMode>(emu: &mut Emu) {
    do_rmw::<_, u8, A, ADDR>(emu, |emu, value| {
        add_io_cycles(emu, 1);
        let a = A::trunc_u16(emu.cpu.regs.a);
        emu.cpu.regs.psw.set_zero((value & a).is_zero());
        value | a
    });
}

pub(super) fn trb<A: RegSize, const ADDR: AddrMode>(emu: &mut Emu) {
    do_rmw::<_, u8, A, ADDR>(emu, |emu, value| {
        add_io_cycles(emu, 1);
        let a = A::trunc_u16(emu.cpu.regs.a);
        emu.cpu.regs.psw.set_zero((value & a).is_zero());
        value & !a
    });
}

pub(super) fn cpx<I: RegSize, const ADDR: AddrMode>(emu: &mut Emu) {
    do_compare::<I, I, ADDR>(emu, emu.cpu.regs.x);
}

pub(super) fn cpy<I: RegSize, const ADDR: AddrMode>(emu: &mut Emu) {
    do_compare::<I, I, ADDR>(emu, emu.cpu.regs.y);
}

pub(super) fn inx<I: RegSize>(emu: &mut Emu) {
    emu.cpu.regs.x = do_inc(emu, I::trunc_u16(emu.cpu.regs.x)).as_zext_u16();
}

pub(super) fn iny<I: RegSize>(emu: &mut Emu) {
    emu.cpu.regs.y = do_inc(emu, I::trunc_u16(emu.cpu.regs.y)).as_zext_u16();
}

pub(super) fn dex<I: RegSize>(emu: &mut Emu) {
    emu.cpu.regs.x = do_dec(emu, I::trunc_u16(emu.cpu.regs.x)).as_zext_u16();
}

pub(super) fn dey<I: RegSize>(emu: &mut Emu) {
    emu.cpu.regs.y = do_dec(emu, I::trunc_u16(emu.cpu.regs.y)).as_zext_u16();
}