mod instrs;
use instrs::*;
mod common;
use common::*;

use crate::apu::Apu;
use core::{mem::replace, ops::Range};

static INSTR_TABLE: [fn(&mut Context); 0x100] =
    include!(concat!(env!("OUT_DIR"), "/instr_table_spc700_disasm.rs"));

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Instr {
    pub addr: u16,
    pub opcode: String,
    pub op_addr: String,
}

struct Context<'a> {
    apu: &'a mut Apu,
    pc: u16,
    direct_page_base: Option<u16>,
    next_instr: Instr,
}

impl<'a> Context<'a> {
    fn from_apu_state_and_addr(apu: &'a mut Apu, addr: u16) -> Self {
        Context {
            pc: addr,
            direct_page_base: Some(apu.spc700.regs.direct_page_base()),
            next_instr: Instr {
                addr,
                opcode: String::new(),
                op_addr: String::new(),
            },
            apu,
        }
    }

    fn disassemble_while(
        mut self,
        result: &mut Vec<Instr>,
        mut cond: impl FnMut(&Self, &Vec<Instr>) -> bool,
    ) {
        while cond(&self, result) {
            let instr = self.consume_imm_8();
            INSTR_TABLE[instr as usize](&mut self);
            result.push(replace(
                &mut self.next_instr,
                Instr {
                    addr: self.pc,
                    opcode: String::new(),
                    op_addr: String::new(),
                },
            ));
        }
    }

    fn disassemble_single(mut self) -> Instr {
        let instr = self.consume_imm_8();
        INSTR_TABLE[instr as usize](&mut self);
        self.next_instr
    }
}

pub fn disassemble_range_with_apu_state(apu: &mut Apu, addrs: Range<u16>, result: &mut Vec<Instr>) {
    Context::from_apu_state_and_addr(apu, addrs.start)
        .disassemble_while(result, |ctx, _| (ctx.pc as u16) < addrs.end);
}

pub fn disassemble_count_with_apu_state(
    apu: &mut Apu,
    start_addr: u16,
    count: usize,
    result: &mut Vec<Instr>,
) {
    Context::from_apu_state_and_addr(apu, start_addr)
        .disassemble_while(result, |_, result| result.len() < count);
}

pub fn disassemble_single_with_apu_state(apu: &mut Apu, addr: u16) -> Instr {
    Context::from_apu_state_and_addr(apu, addr).disassemble_single()
}
