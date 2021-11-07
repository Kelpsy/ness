use super::common::{add_io_cycles, set_nz, RegSize};
use crate::emu::Emu;

pub(super) fn xba(emu: &mut Emu) {
    emu.cpu.regs.a = emu.cpu.regs.a.swap_bytes();
    set_nz(emu, emu.cpu.regs.a as u8);
    add_io_cycles(emu, 2);
}

pub(super) fn tcs(emu: &mut Emu) {
    emu.cpu.regs.sp = emu.cpu.regs.a;
    add_io_cycles(emu, 1);
}

pub(super) fn tsc(emu: &mut Emu) {
    emu.cpu.regs.a = emu.cpu.regs.sp;
    set_nz(emu, emu.cpu.regs.a);
    add_io_cycles(emu, 1);
}

pub(super) fn tcd(emu: &mut Emu) {
    emu.cpu.regs.direct_page_offset = emu.cpu.regs.a;
    set_nz(emu, emu.cpu.regs.a);
    add_io_cycles(emu, 1);
}

pub(super) fn tdc(emu: &mut Emu) {
    emu.cpu.regs.a = emu.cpu.regs.direct_page_offset;
    set_nz(emu, emu.cpu.regs.a);
    add_io_cycles(emu, 1);
}

pub(super) fn tax<I: RegSize>(emu: &mut Emu) {
    let result = I::trunc_u16(emu.cpu.regs.a);
    emu.cpu.regs.x = result.as_zext_u16();
    set_nz(emu, result);
    add_io_cycles(emu, 1);
}

pub(super) fn txa<A: RegSize>(emu: &mut Emu) {
    let result = A::trunc_u16(emu.cpu.regs.x);
    result.update_u16_low(&mut emu.cpu.regs.a);
    set_nz(emu, result);
    add_io_cycles(emu, 1);
}

pub(super) fn tay<I: RegSize>(emu: &mut Emu) {
    let result = I::trunc_u16(emu.cpu.regs.a);
    emu.cpu.regs.y = result.as_zext_u16();
    set_nz(emu, result);
    add_io_cycles(emu, 1);
}

pub(super) fn tya<A: RegSize>(emu: &mut Emu) {
    let result = A::trunc_u16(emu.cpu.regs.y);
    result.update_u16_low(&mut emu.cpu.regs.a);
    set_nz(emu, result);
    add_io_cycles(emu, 1);
}

pub(super) fn txy<I: RegSize>(emu: &mut Emu) {
    emu.cpu.regs.y = emu.cpu.regs.x;
    set_nz::<I>(emu, I::trunc_u16(emu.cpu.regs.y));
    add_io_cycles(emu, 1);
}

pub(super) fn tyx<I: RegSize>(emu: &mut Emu) {
    emu.cpu.regs.x = emu.cpu.regs.y;
    set_nz::<I>(emu, I::trunc_u16(emu.cpu.regs.x));
    add_io_cycles(emu, 1);
}

pub(super) fn txs(emu: &mut Emu) {
    emu.cpu.regs.sp = emu.cpu.regs.x;
    add_io_cycles(emu, 1);
}

pub(super) fn tsx<I: RegSize>(emu: &mut Emu) {
    let result = I::trunc_u16(emu.cpu.regs.sp);
    emu.cpu.regs.x = result.as_zext_u16();
    set_nz(emu, result);
    add_io_cycles(emu, 1);
}
