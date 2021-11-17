use super::{common::AddrMode, Context};
use core::fmt::Write;

pub(super) fn set_direct_page<const VALUE: bool>(ctx: &mut Context) {
    ctx.direct_page_base = Some(if VALUE { 0x100 } else { 0 });
    ctx.next_instr.opcode = if VALUE { "SETP" } else { "CLRP" }.to_string();
}

pub(super) fn branch<const COND: &'static str>(ctx: &mut Context) {
    ctx.next_instr.opcode = format!("B{} ", COND);
    ctx.handle_branch_offset();
}

pub(super) fn branch_bit<const SET: bool, const BIT: u8>(ctx: &mut Context) {
    ctx.next_instr.opcode = if SET { "BBS " } else { "BBC " }.to_string();
    ctx.handle_direct_addr("", "");
    write!(ctx.next_instr.opcode, ".{}, ", BIT).unwrap();
    ctx.next_instr.op_addr.push_str(", ");
    ctx.handle_branch_offset();
}

pub(super) fn cbne<const ADDR: AddrMode>(ctx: &mut Context) {
    ctx.next_instr.opcode = "CBNE ".to_string();
    ctx.handle_mem_op::<ADDR>();
    ctx.next_instr.opcode.push_str(", ");
    ctx.next_instr.op_addr.push_str(", ");
    ctx.handle_branch_offset();
}

pub(super) fn dbnz_direct(ctx: &mut Context) {
    ctx.next_instr.opcode = "DBNZ ".to_string();
    ctx.handle_direct_addr("", "");
    ctx.next_instr.opcode.push_str(", ");
    ctx.next_instr.op_addr.push_str(", ");
    ctx.handle_branch_offset();
}

pub(super) fn dbnz_y(ctx: &mut Context) {
    ctx.next_instr.opcode = "DBNZ Y, ".to_string();
    ctx.handle_branch_offset();
}

pub(super) fn jmp_absolute(ctx: &mut Context) {
    ctx.next_instr.opcode = "JMP ".to_string();
    ctx.handle_absolute_addr("", "");
}

pub(super) fn jmp_abs_x_indirect(ctx: &mut Context) {
    let addr = ctx.read_absolute_addr();
    ctx.next_instr.opcode = format!("JMP [!${:04X}+X]", addr);
    ctx.next_instr.op_addr = format!("[{:04X} + X]", addr);
}

pub(super) fn call(ctx: &mut Context) {
    ctx.next_instr.opcode = "CALL ".to_string();
    ctx.handle_absolute_addr("", "");
}

pub(super) fn pcall(ctx: &mut Context) {
    let offset = ctx.consume_imm_8();
    ctx.next_instr.opcode = format!("PCALL ${:02X}", offset);
    ctx.next_instr.op_addr = format!("{:04X}", 0xFF00 | offset as u16);
}

pub(super) fn tcall<const INDEX: u8>(ctx: &mut Context) {
    ctx.next_instr.opcode = format!("TCALL {}", INDEX);
    ctx.next_instr.op_addr = format!("{:04X}", 0xFFDE - 2 * INDEX as u16);
}

pub(super) fn modify_bit<const OP: &'static str, const BIT: u8>(ctx: &mut Context) {
    ctx.next_instr.opcode = format!("{} ", OP);
    ctx.handle_direct_addr("", "");
    write!(ctx.next_instr.opcode, ".{}", BIT).unwrap();
}

pub(super) fn test_modify_bit<const SET: bool>(ctx: &mut Context) {
    ctx.next_instr.opcode = if SET { "TSET1 " } else { "TCLR1 " }.to_string();
    ctx.handle_absolute_addr("", "");
}

pub(super) fn op_reg_mem<const OP: &'static str, const REG: &'static str, const ADDR: AddrMode>(
    ctx: &mut Context,
) {
    ctx.next_instr.opcode = format!("{} {}, ", OP, REG);
    ctx.handle_mem_op::<ADDR>();
}

pub(super) fn op_mem_reg<const OP: &'static str, const ADDR: AddrMode, const REG: &'static str>(
    ctx: &mut Context,
) {
    ctx.next_instr.opcode = format!("{} ", OP);
    ctx.handle_mem_op::<ADDR>();
    write!(ctx.next_instr.opcode, ", {}", REG).unwrap();
}

pub(super) fn op_direct<const OP: &'static str>(ctx: &mut Context) {
    ctx.next_instr.opcode = format!("{} ", OP);
    let src = ctx.read_direct_addr();
    ctx.handle_direct_addr("", "");
    ctx.next_instr.opcode.push_str(", ");
    ctx.handle_direct_addr_custom(src, "", "");
}

pub(super) fn op_direct_imm<const OP: &'static str>(ctx: &mut Context) {
    ctx.next_instr.opcode = format!("{} ", OP);
    let value = ctx.consume_imm_8();
    ctx.handle_direct_addr("", "");
    write!(ctx.next_instr.opcode, ", #${:02X}", value).unwrap();
}

pub(super) fn op_mem<const OP: &'static str, const ADDR: AddrMode>(ctx: &mut Context) {
    ctx.next_instr.opcode = format!("{} ", OP);
    ctx.handle_mem_op::<ADDR>();
}

pub(super) fn op_carry_mem<const OP: &'static str, const INVERT: bool>(ctx: &mut Context) {
    ctx.next_instr.opcode = format!("{} C, {}", OP, if INVERT { "/" } else { "" });
    let addr_bit = ctx.read_absolute_addr();
    ctx.handle_absolute_addr_custom(addr_bit & 0x1FFF, "", "");
    write!(ctx.next_instr.opcode, ".{}", addr_bit >> 13).unwrap();
}

pub(super) fn mov1_mem_carry(ctx: &mut Context) {
    ctx.next_instr.opcode = "MOV1 ".to_string();
    let addr_bit = ctx.read_absolute_addr();
    ctx.handle_absolute_addr_custom(addr_bit & 0x1FFF, "", "");
    write!(ctx.next_instr.opcode, ".{}, C", addr_bit >> 13).unwrap();
}

pub(super) fn not1(ctx: &mut Context) {
    ctx.next_instr.opcode = "NOT1 ".to_string();
    let addr_bit = ctx.read_absolute_addr();
    ctx.handle_absolute_addr_custom(addr_bit & 0x1FFF, "", "");
    write!(ctx.next_instr.opcode, ".{}", addr_bit >> 13).unwrap();
}

pub(super) fn ya_mem_op<const OP: &'static str>(ctx: &mut Context) {
    ctx.next_instr.opcode = format!("{} YA, ", OP);
    ctx.handle_direct_addr("", "");
}

pub(super) fn movw_mem_ya(ctx: &mut Context) {
    ctx.next_instr.opcode = "MOVW ".to_string();
    ctx.handle_direct_addr("", "");
    ctx.next_instr.opcode.push_str(", YA");
}

pub(super) fn modify_direct_word<const OP: &'static str>(ctx: &mut Context) {
    ctx.next_instr.opcode = format!("{} ", OP);
    ctx.handle_direct_addr("", "");
}

pub(super) fn raw<const OP: &'static str>(ctx: &mut Context) {
    ctx.next_instr.opcode = OP.to_string();
}
