use super::common::{
    add_io_cycles, consume_imm, do_addr_mode_read, do_addr_mode_write, pull, push, read_16_bank0,
    read_8, read_direct_addr, set_nz, write_8, AddrMode, RegSize,
};
use crate::{cpu::regs::Psw, emu::Emu};

pub(super) fn ldx<I: RegSize, const ADDR: AddrMode>(emu: &mut Emu) {
    let result = do_addr_mode_read::<I, I, ADDR>(emu);
    emu.cpu.regs.x = result.as_zext_u16();
    set_nz(emu, result);
}

pub(super) fn ldy<I: RegSize, const ADDR: AddrMode>(emu: &mut Emu) {
    let result = do_addr_mode_read::<I, I, ADDR>(emu);
    emu.cpu.regs.y = result.as_zext_u16();
    set_nz(emu, result);
}

pub(super) fn stx<I: RegSize, const ADDR: AddrMode>(emu: &mut Emu) {
    do_addr_mode_write::<I, I, ADDR>(emu, I::trunc_u16(emu.cpu.regs.x));
}

pub(super) fn sty<I: RegSize, const ADDR: AddrMode>(emu: &mut Emu) {
    do_addr_mode_write::<I, I, ADDR>(emu, I::trunc_u16(emu.cpu.regs.y));
}

pub(super) fn stz<A: RegSize, I: RegSize, const ADDR: AddrMode>(emu: &mut Emu) {
    do_addr_mode_write::<I, A, ADDR>(emu, A::trunc_u16(0));
}

pub(super) fn mvp<I: RegSize>(emu: &mut Emu) {
    let opcode_base_addr = emu.cpu.regs.pc.wrapping_sub(1);
    emu.cpu.regs.pc = emu.cpu.regs.pc.wrapping_add(2);
    let opcode_addr = opcode_base_addr as u32 | emu.cpu.regs.code_bank_base();
    let dst_bank_addr = opcode_base_addr.wrapping_add(1) as u32 | emu.cpu.regs.code_bank_base();
    let src_bank_addr = opcode_base_addr.wrapping_add(2) as u32 | emu.cpu.regs.code_bank_base();
    loop {
        let dst_bank = read_8(emu, dst_bank_addr);
        let src_bank = read_8(emu, src_bank_addr);
        let value = read_8(emu, emu.cpu.regs.x as u32 | (src_bank as u32) << 16);
        write_8(emu, emu.cpu.regs.y as u32 | (dst_bank as u32) << 16, value);
        add_io_cycles(emu, 2);
        emu.cpu.regs.x = I::trunc_u16(emu.cpu.regs.x.wrapping_sub(1)).as_zext_u16();
        emu.cpu.regs.y = I::trunc_u16(emu.cpu.regs.y.wrapping_sub(1)).as_zext_u16();
        emu.cpu.regs.a = emu.cpu.regs.a.wrapping_sub(1);
        if emu.cpu.regs.a == 0xFFFF {
            break;
        }
        let _opcode = read_8(emu, opcode_addr);
    }
}

pub(super) fn mvn<I: RegSize>(emu: &mut Emu) {
    let opcode_base_addr = emu.cpu.regs.pc.wrapping_sub(1);
    emu.cpu.regs.pc = emu.cpu.regs.pc.wrapping_add(2);
    let opcode_addr = opcode_base_addr as u32 | emu.cpu.regs.code_bank_base();
    let dst_bank_addr = opcode_base_addr.wrapping_add(1) as u32 | emu.cpu.regs.code_bank_base();
    let src_bank_addr = opcode_base_addr.wrapping_add(2) as u32 | emu.cpu.regs.code_bank_base();
    loop {
        let dst_bank = read_8(emu, dst_bank_addr);
        let src_bank = read_8(emu, src_bank_addr);
        let value = read_8(emu, emu.cpu.regs.x as u32 | (src_bank as u32) << 16);
        write_8(emu, emu.cpu.regs.y as u32 | (dst_bank as u32) << 16, value);
        add_io_cycles(emu, 2);
        emu.cpu.regs.x = I::trunc_u16(emu.cpu.regs.x.wrapping_add(1)).as_zext_u16();
        emu.cpu.regs.y = I::trunc_u16(emu.cpu.regs.y.wrapping_add(1)).as_zext_u16();
        emu.cpu.regs.a = emu.cpu.regs.a.wrapping_sub(1);
        if emu.cpu.regs.a == 0xFFFF {
            break;
        }
        let _opcode = read_8(emu, opcode_addr);
    }
}

pub(super) fn pha<A: RegSize>(emu: &mut Emu) {
    add_io_cycles(emu, 1);
    push(emu, A::trunc_u16(emu.cpu.regs.a));
}

pub(super) fn phx<I: RegSize>(emu: &mut Emu) {
    add_io_cycles(emu, 1);
    push(emu, I::trunc_u16(emu.cpu.regs.x));
}

pub(super) fn phy<I: RegSize>(emu: &mut Emu) {
    add_io_cycles(emu, 1);
    push(emu, I::trunc_u16(emu.cpu.regs.y));
}

pub(super) fn php(emu: &mut Emu) {
    add_io_cycles(emu, 1);
    push::<u8>(emu, emu.cpu.regs.psw.0);
}

pub(super) fn phb(emu: &mut Emu) {
    add_io_cycles(emu, 1);
    push::<u8>(emu, emu.cpu.regs.data_bank());
}

pub(super) fn phk(emu: &mut Emu) {
    add_io_cycles(emu, 1);
    push::<u8>(emu, emu.cpu.regs.code_bank());
}

pub(super) fn phd(emu: &mut Emu) {
    add_io_cycles(emu, 1);
    push::<u16>(emu, emu.cpu.regs.direct_page_offset);
}

pub(super) fn pea(emu: &mut Emu) {
    let value = consume_imm::<u16>(emu);
    push(emu, value);
}

pub(super) fn pei(emu: &mut Emu) {
    let indirect_addr = read_direct_addr(emu);
    let addr = read_16_bank0(emu, indirect_addr);
    push(emu, addr);
}

pub(super) fn per(emu: &mut Emu) {
    let offset = consume_imm::<u16>(emu);
    add_io_cycles(emu, 1);
    push(emu, emu.cpu.regs.pc.wrapping_add(offset));
}

pub(super) fn pla<A: RegSize>(emu: &mut Emu) {
    add_io_cycles(emu, 2);
    let result = pull::<A>(emu);
    result.update_u16_low(&mut emu.cpu.regs.a);
    set_nz(emu, result);
}

pub(super) fn plx<I: RegSize>(emu: &mut Emu) {
    add_io_cycles(emu, 2);
    let result = pull::<I>(emu);
    emu.cpu.regs.x = result.as_zext_u16();
    set_nz(emu, result);
}

pub(super) fn ply<I: RegSize>(emu: &mut Emu) {
    add_io_cycles(emu, 2);
    let result = pull::<I>(emu);
    emu.cpu.regs.y = result.as_zext_u16();
    set_nz(emu, result);
}

pub(super) fn plp(emu: &mut Emu) {
    add_io_cycles(emu, 2);
    let result = pull::<u8>(emu);
    emu.cpu.regs.set_psw(Psw(result));
    // TODO: Update IRQs enabled
}

pub(super) fn plb(emu: &mut Emu) {
    add_io_cycles(emu, 2);
    let result = pull::<u8>(emu);
    emu.cpu.regs.set_data_bank(result);
    set_nz(emu, result);
}

pub(super) fn pld(emu: &mut Emu) {
    add_io_cycles(emu, 2);
    let result = pull::<u16>(emu);
    emu.cpu.regs.direct_page_offset = result;
    set_nz(emu, result);
}
