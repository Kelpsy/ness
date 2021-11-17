mod instrs;
use instrs::*;
mod common;
use common::*;

use crate::emu::Emu;
use core::{mem::replace, ops::Range};

static INSTR_TABLE: [fn(&mut Context); 0x400] =
    include!(concat!(env!("OUT_DIR"), "/instr_table_65c816_disasm.rs"));

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Instr {
    pub addr: u32,
    pub opcode: String,
    pub op_addr: String,
    pub comment: String,
    pub followed_by_bank_boundary_crossing: bool,
}

struct Context<'a> {
    emu: &'a mut Emu,
    pc: u16,
    direct_page_offset: Option<u16>,
    a_is_8_bit: bool,
    index_regs_are_8_bit: bool,
    code_bank: u8,
    psw_lut_base: u16,
    code_bank_base: u32,
    data_bank_base: Option<u32>,
    next_instr: Instr,
}

impl<'a> Context<'a> {
    fn from_emu_state_and_addr(emu: &'a mut Emu, addr: u32) -> Self {
        let mut ctx = Context {
            pc: addr as u16,
            direct_page_offset: Some(emu.cpu.regs.direct_page_offset),
            a_is_8_bit: emu.cpu.regs.psw.a_is_8_bit(),
            index_regs_are_8_bit: emu.cpu.regs.psw.index_regs_are_8_bit(),
            code_bank: (addr >> 16) as u8,
            psw_lut_base: 0,
            code_bank_base: addr & 0xFF_0000,
            data_bank_base: Some((emu.cpu.regs.data_bank() as u32) << 16),
            next_instr: Instr {
                addr,
                opcode: String::new(),
                op_addr: String::new(),
                comment: String::new(),
                followed_by_bank_boundary_crossing: false,
            },
            emu,
        };
        ctx.update_psw_lut_base();
        ctx
    }

    fn update_psw_lut_base(&mut self) {
        self.psw_lut_base = (self.a_is_8_bit as u16) << 9 | (self.index_regs_are_8_bit as u16) << 8;
    }

    fn disassemble_while(
        mut self,
        result: &mut Vec<Instr>,
        mut cond: impl FnMut(&Self, &Vec<Instr>) -> bool,
    ) {
        while cond(&self, result) {
            let instr = self.consume_imm::<u8>();
            unsafe {
                INSTR_TABLE.get_unchecked(instr as usize | self.psw_lut_base as usize)(&mut self)
            };
            if self.next_instr.followed_by_bank_boundary_crossing {
                self.code_bank = self.code_bank.wrapping_add(1);
            }
            result.push(replace(
                &mut self.next_instr,
                Instr {
                    addr: self.pc as u32 | self.code_bank_base,
                    opcode: String::new(),
                    op_addr: String::new(),
                    comment: String::new(),
                    followed_by_bank_boundary_crossing: false,
                },
            ));
        }
    }

    fn disassemble_single(mut self) -> Instr {
        let instr = self.consume_imm::<u8>();
        unsafe {
            INSTR_TABLE.get_unchecked(instr as usize | self.psw_lut_base as usize)(&mut self)
        };
        self.next_instr
    }
}

pub fn disassemble_range_with_emu_state(emu: &mut Emu, addrs: Range<u32>, result: &mut Vec<Instr>) {
    Context::from_emu_state_and_addr(emu, addrs.start).disassemble_while(result, |ctx, _| {
        ctx.pc as u32 | ctx.code_bank_base < addrs.end
    });
}

pub fn disassemble_count_with_emu_state(
    emu: &mut Emu,
    start_addr: u32,
    count: usize,
    result: &mut Vec<Instr>,
) {
    Context::from_emu_state_and_addr(emu, start_addr)
        .disassemble_while(result, |_, result| result.len() < count);
}

pub fn disassemble_single_with_emu_state(emu: &mut Emu, addr: u32) -> Instr {
    Context::from_emu_state_and_addr(emu, addr).disassemble_single()
}
