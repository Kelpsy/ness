use super::common::{add_io_cycles, consume_imm, pull, push, read_16_bank0, read_8, JumpAddr};
use crate::{cpu::regs::Psw, emu::Emu};

fn do_cond_branch(emu: &mut Emu, cond: impl FnOnce(Psw) -> bool) {
    let offset = consume_imm::<u8>(emu) as i8;
    if cond(emu.cpu.regs.psw) {
        add_io_cycles(emu, 1);
        emu.cpu.regs.pc = emu.cpu.regs.pc.wrapping_add(offset as u16);
    }
}

pub fn bra(emu: &mut Emu) {
    do_cond_branch(emu, |_| true);
}

pub fn b_cond<const BIT: u8, const SET: bool>(emu: &mut Emu) {
    do_cond_branch(emu, |psw| (psw.0 & 1 << BIT != 0) == SET);
}

pub fn brl(emu: &mut Emu) {
    let offset = consume_imm::<u16>(emu) as i16;
    add_io_cycles(emu, 1);
    emu.cpu.regs.pc = emu.cpu.regs.pc.wrapping_add(offset as u16);
}

pub fn jmp<const SUBROUTINE: bool, const ADDR: JumpAddr>(emu: &mut Emu) {
    match ADDR {
        JumpAddr::Absolute => {
            let new_pc = consume_imm::<u16>(emu);
            if SUBROUTINE {
                add_io_cycles(emu, 1);
                push(emu, emu.cpu.regs.pc.wrapping_sub(1));
            }
            emu.cpu.regs.pc = new_pc;
        }
        JumpAddr::AbsoluteLong => {
            let new_pc = consume_imm::<u16>(emu);
            if SUBROUTINE {
                push(emu, emu.cpu.regs.code_bank());
                add_io_cycles(emu, 1);
            }
            let new_code_bank = consume_imm::<u8>(emu);
            if SUBROUTINE {
                push(emu, emu.cpu.regs.pc.wrapping_sub(1));
            }
            emu.cpu.regs.pc = new_pc;
            emu.cpu.regs.set_code_bank(new_code_bank);
        }
        JumpAddr::AbsoluteIndirect => {
            let indirect_addr = consume_imm::<u16>(emu);
            let new_pc = read_16_bank0(emu, indirect_addr);
            emu.cpu.regs.pc = new_pc;
        }
        JumpAddr::AbsoluteIndirectLong => {
            let indirect_addr = consume_imm::<u16>(emu);
            let new_pc = read_16_bank0(emu, indirect_addr);
            let new_code_bank = read_8(emu, indirect_addr.wrapping_add(2) as u32);
            emu.cpu.regs.pc = new_pc;
            emu.cpu.regs.set_code_bank(new_code_bank);
        }
        JumpAddr::AbsoluteXIndirect => {
            let indirect_addr = if SUBROUTINE {
                let low = consume_imm::<u8>(emu);
                push(emu, emu.cpu.regs.pc);
                let high = consume_imm::<u8>(emu);
                low as u16 | (high as u16) << 8
            } else {
                consume_imm::<u16>(emu)
            }
            .wrapping_add(emu.cpu.regs.x);
            add_io_cycles(emu, 1);
            // NOTE: Absolute indexed indirect mode reads the indirect address from the program bank
            let new_pc = read_8(emu, indirect_addr as u32 | emu.cpu.regs.code_bank_base()) as u16
                | (read_8(
                    emu,
                    indirect_addr.wrapping_add(1) as u32 | emu.cpu.regs.code_bank_base(),
                ) as u16)
                    << 8;
            emu.cpu.regs.pc = new_pc;
        }
    }
}

pub fn rts(emu: &mut Emu) {
    add_io_cycles(emu, 2);
    let new_pc = pull::<u16>(emu).wrapping_add(1);
    add_io_cycles(emu, 1);
    emu.cpu.regs.pc = new_pc;
}

pub fn rtl(emu: &mut Emu) {
    add_io_cycles(emu, 2);
    let new_pc = pull::<u16>(emu).wrapping_add(1);
    let new_code_bank = pull::<u8>(emu);
    emu.cpu.regs.pc = new_pc;
    emu.cpu.regs.set_code_bank(new_code_bank);
}
