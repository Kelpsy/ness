use super::{
    common::{AddrMode, JumpAddr, RegSize},
    Context,
};

pub(super) fn mem_op<A: RegSize, const OP: &'static str, const ADDR: AddrMode>(ctx: &mut Context) {
    ctx.next_instr.opcode = format!("{} ", OP);
    ctx.handle_mem_op::<A, ADDR>();
}

pub(super) fn b_cond<const COND: &'static str>(ctx: &mut Context) {
    let offset = ctx.consume_imm::<u8>() as i8;
    ctx.next_instr.opcode = format!(
        "B{} ${}{:02X}",
        COND,
        if offset < 0 { "-" } else { "" },
        if offset < 0 { -offset } else { offset }
    );
    ctx.next_instr.op_addr = format!(
        "{:06X}",
        ctx.code_bank_base | ctx.pc.wrapping_add(offset as u16) as u32
    );
}

pub(super) fn brl(ctx: &mut Context) {
    let offset = ctx.consume_imm::<u16>() as i16;
    ctx.next_instr.opcode = format!(
        "BRL ${}{:04X}",
        if offset < 0 { "-" } else { "" },
        if offset < 0 { -offset } else { offset }
    );
    ctx.next_instr.op_addr = format!(
        "{:06X}",
        ctx.code_bank_base | ctx.pc.wrapping_add(offset as u16) as u32
    );
}

pub(super) fn jmp<const SUBROUTINE: bool, const ADDR: JumpAddr>(ctx: &mut Context) {
    let instr_name = if matches!(
        ADDR,
        JumpAddr::AbsoluteLong | JumpAddr::AbsoluteIndirectLong
    ) {
        if SUBROUTINE {
            "JSL"
        } else {
            "JML"
        }
    } else if SUBROUTINE {
        "JSR"
    } else {
        "JMP"
    };
    match ADDR {
        JumpAddr::Absolute => {
            let new_pc = ctx.read_absolute_short_addr();
            let long_addr = ctx.code_bank_base | new_pc as u32;
            ctx.next_instr.opcode = format!("{} ${:04X}", instr_name, new_pc);
            ctx.next_instr.op_addr = format!("{:06X}", long_addr);
        }
        JumpAddr::AbsoluteLong => {
            let long_addr = ctx.read_absolute_long_addr();
            ctx.next_instr.opcode = format!("{} ${:06X}", instr_name, long_addr);
            ctx.next_instr.op_addr = format!("{:06X}", long_addr);
        }
        JumpAddr::AbsoluteIndirect => {
            let indirect_addr = ctx.read_absolute_short_addr();
            let new_pc = ctx.read_indirect_short_addr(indirect_addr);
            let long_addr = ctx.code_bank_base | new_pc as u32;
            ctx.next_instr.opcode = format!("{} (${:04X})", instr_name, indirect_addr);
            ctx.next_instr.op_addr = format!("{:06X}", long_addr);
        }
        JumpAddr::AbsoluteIndirectLong => {
            let indirect_addr = ctx.read_absolute_short_addr();
            let long_addr = ctx.read_indirect_long_addr(indirect_addr);
            ctx.next_instr.opcode = format!("{} [${:04X}]", instr_name, indirect_addr);
            ctx.next_instr.op_addr = format!("{:06X}", long_addr);
        }
        JumpAddr::AbsoluteXIndirect => {
            let indirect_unindexed_addr = ctx.read_absolute_short_addr();
            ctx.next_instr.opcode = format!("{} (${:04X},X)", instr_name, indirect_unindexed_addr);
            ctx.next_instr.op_addr = format!(
                "({:06X} + {:04X} + X)",
                ctx.code_bank_base, indirect_unindexed_addr
            );
        }
    }
}

pub(super) fn per(ctx: &mut Context) {
    let offset = ctx.consume_imm::<u16>() as i16;
    ctx.next_instr.opcode = format!(
        "PER {}{:04X}",
        if offset < 0 { "-" } else { "" },
        if offset < 0 { -offset } else { offset }
    );
    ctx.next_instr.op_addr = format!("{:04X}", (ctx.pc as u16).wrapping_add(offset as u16));
}

pub(super) fn move_block<const NEGATIVE: bool>(ctx: &mut Context) {
    let dst_bank = ctx.consume_imm::<u8>();
    let src_bank = ctx.consume_imm::<u8>();
    ctx.next_instr.opcode = format!(
        "{} {:02X},{:02X}",
        if NEGATIVE { "MVN" } else { "MVP" },
        src_bank,
        dst_bank
    );
}

pub(super) fn sep(ctx: &mut Context) {
    let mask = ctx.consume_imm::<u8>();
    ctx.index_regs_are_8_bit |= mask & 1 << 4 != 0;
    ctx.a_is_8_bit |= mask & 1 << 5 != 0;
    ctx.update_psw_lut_base();
    ctx.next_instr.opcode = format!("SEP #%{:08b}", mask);
}

pub(super) fn rep(ctx: &mut Context) {
    let mask = ctx.consume_imm::<u8>();
    ctx.index_regs_are_8_bit &= mask & 1 << 4 == 0;
    ctx.a_is_8_bit &= mask & 1 << 5 == 0;
    ctx.update_psw_lut_base();
    ctx.next_instr.opcode = format!("REP #%{:08b}", mask);
}

pub(super) fn raw<const OP: &'static str>(ctx: &mut Context) {
    ctx.next_instr.opcode = OP.to_string();
}

pub(super) fn marker_byte<const OP: &'static str>(ctx: &mut Context) {
    let imm = ctx.consume_imm::<u8>();
    ctx.next_instr.opcode = format!("{} #${:02X}", OP, imm);
}
