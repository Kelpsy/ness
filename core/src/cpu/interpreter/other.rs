use super::common::{add_io_cycles, consume_imm, jump_to_exc_vector, pull, push};
use crate::{cpu::regs::Psw, emu::Emu};

pub(super) fn sec(emu: &mut Emu) {
    emu.cpu.regs.psw.set_carry(true);
}

pub(super) fn sed(emu: &mut Emu) {
    emu.cpu
        .regs
        .set_psw(emu.cpu.regs.psw.with_decimal_mode(true));
}

pub(super) fn sei(emu: &mut Emu) {
    emu.cpu.regs.psw.set_irqs_disabled(true);
    // TODO: Update IRQs enabled
}

pub(super) fn clc(emu: &mut Emu) {
    emu.cpu.regs.psw.set_carry(false);
}

pub(super) fn cld(emu: &mut Emu) {
    emu.cpu
        .regs
        .set_psw(emu.cpu.regs.psw.with_decimal_mode(false));
}

pub(super) fn cli(emu: &mut Emu) {
    emu.cpu.regs.psw.set_irqs_disabled(false);
    // TODO: Update IRQs enabled
}

pub(super) fn clv(emu: &mut Emu) {
    emu.cpu.regs.psw.set_overflow(false);
}

pub(super) fn sep(emu: &mut Emu) {
    let mask = consume_imm::<u8>(emu);
    emu.cpu.regs.set_psw(Psw(emu.cpu.regs.psw.0 | mask));
    // TODO: Update IRQs enabled
    add_io_cycles(emu, 1);
}

pub(super) fn rep(emu: &mut Emu) {
    let mask = consume_imm::<u8>(emu);
    emu.cpu.regs.set_psw(Psw(emu.cpu.regs.psw.0 & !mask));
    // TODO: Update IRQs enabled
    add_io_cycles(emu, 1);
}

pub(super) fn xce(emu: &mut Emu) {
    let new_value = emu.cpu.regs.psw.carry();
    emu.cpu.regs.psw.set_carry(emu.cpu.regs.emulation_mode());
    emu.cpu.regs.set_emulation_mode::<false>(new_value);
}

pub(super) fn rti(emu: &mut Emu) {
    add_io_cycles(emu, 2);
    let new_psw = pull::<u8>(emu);
    let new_pc = pull::<u16>(emu);
    let new_code_bank = pull::<u8>(emu);
    emu.cpu.regs.set_psw(Psw(new_psw));
    // TODO: Update IRQs enabled
    emu.cpu.regs.pc = new_pc;
    emu.cpu.regs.set_code_bank(new_code_bank);
}

pub(super) fn brk(emu: &mut Emu) {
    #[cfg(feature = "log")]
    slog::info!(
        emu.cpu.logger,
        "BRK encountered @ {:#08X}",
        emu.cpu.regs.pc.wrapping_sub(1) as u32 | emu.cpu.regs.code_bank_base()
    );
    let _signature = consume_imm::<u8>(emu);
    push(emu, emu.cpu.regs.code_bank());
    push(emu, emu.cpu.regs.pc);
    push(emu, emu.cpu.regs.psw.0);
    jump_to_exc_vector(emu, 0xFFE6);
}

pub(super) fn nop(emu: &mut Emu) {
    add_io_cycles(emu, 1);
}

pub(super) fn wai(emu: &mut Emu) {
    todo!("wai");
}

pub(super) fn cop(emu: &mut Emu) {
    let _signature = consume_imm::<u8>(emu);
    push(emu, emu.cpu.regs.code_bank());
    push(emu, emu.cpu.regs.pc);
    push(emu, emu.cpu.regs.psw.0);
    jump_to_exc_vector(emu, 0xFFE4);
}

pub(super) fn stp(emu: &mut Emu) {
    #[cfg(feature = "log")]
    slog::warn!(
        emu.cpu.logger,
        "STP encountered @ {:#08X}",
        emu.cpu.regs.pc.wrapping_sub(1) as u32 | emu.cpu.regs.code_bank_base()
    );
    emu.cpu.stopped = true;
}

pub(super) fn wdm(emu: &mut Emu) {
    #[cfg(feature = "log")]
    slog::warn!(
        emu.cpu.logger,
        "WDM encountered @ {:#08X}",
        emu.cpu.regs.pc.wrapping_sub(1) as u32 | emu.cpu.regs.code_bank_base()
    );
    let _dummy = consume_imm::<u8>(emu);
}
