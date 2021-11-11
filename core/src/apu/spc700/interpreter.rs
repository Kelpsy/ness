// TODO: All sub-instruction timings here are almost completely made up, find an authoritative doc

mod alu;
use alu::*;
mod transfers;
use transfers::*;
mod branches;
use branches::*;
mod other;
use other::*;
mod common;

use crate::{apu::Apu, schedule::Timestamp};
use common::{consume_imm_8, AddrMode, MemOrReg, Reg};

static INSTR_TABLE: [fn(&mut Apu); 0x100] =
    include!(concat!(env!("OUT_DIR"), "/instr_table_spc700.rs"));

pub fn run(apu: &mut Apu, end_timestamp: Timestamp) {
    while apu.spc700.cur_timestamp < end_timestamp {
        let instr = consume_imm_8(apu);
        INSTR_TABLE[instr as usize](apu);
    }
}
