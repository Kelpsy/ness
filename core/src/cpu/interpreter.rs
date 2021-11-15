mod common;
use common::*;

mod alu;
use alu::*;
mod branches;
use branches::*;
mod mem;
use mem::*;
mod other;
use other::*;
mod transfers;
use transfers::*;

use super::dma;
use crate::emu::Emu;
use common::jump_to_exc_vector;

pub fn soft_reset(emu: &mut Emu) {
    emu.cpu.stopped = false;
    emu.cpu.regs.set_psw(
        emu.cpu
            .regs
            .psw
            .with_a_is_8_bit(true)
            .with_index_regs_are_8_bit(true),
    );
    emu.cpu.regs.set_emulation_mode::<true>(true);
    emu.cpu.regs.direct_page_offset = 0;
    emu.cpu.regs.sp = 0x1FC;
    emu.cpu.regs.set_data_bank(0);
    jump_to_exc_vector(emu, 0xFFFC);
}

static INSTR_TABLE: [fn(&mut Emu); 0x800] =
    include!(concat!(env!("OUT_DIR"), "/instr_table_65c816.rs"));

#[inline]
pub fn run_until_next_event(emu: &mut Emu) {
    while emu.schedule.cur_time < emu.schedule.next_event_time() {
        if let Some(channel) = emu.cpu.dmac.cur_channel {
            dma::Controller::run_dma(emu, channel);
        } else {
            emu.schedule.target_time = emu.schedule.next_event_time();
            if emu.cpu.stopped || emu.cpu.irqs.waiting_for_exception() {
                emu.schedule.cur_time +=
                    (emu.schedule.target_time - emu.schedule.cur_time + 5) / 6 * 6;
                return;
            }
            if emu.cpu.irqs.processing_nmi() {
                emu.cpu.irqs.acknowledge_nmi();
                push(emu, emu.cpu.regs.code_bank());
                push(emu, emu.cpu.regs.pc);
                push(emu, emu.cpu.regs.psw.0);
                jump_to_exc_vector(emu, 0xFFEA);
            } else if emu.cpu.irqs.processing_irq() {
                push(emu, emu.cpu.regs.code_bank());
                push(emu, emu.cpu.regs.pc);
                push(emu, emu.cpu.regs.psw.0);
                jump_to_exc_vector(emu, 0xFFEE);
            }
            while emu.schedule.cur_time < emu.schedule.target_time {
                let instr = consume_imm::<u8>(emu);
                unsafe {
                    INSTR_TABLE.get_unchecked(instr as usize | emu.cpu.regs.psw_lut_base() as usize)(
                        emu,
                    )
                };
            }
        }
    }
}
