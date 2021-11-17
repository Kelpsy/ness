use super::Context;
use crate::apu::spc700::bus;
use core::fmt::Write;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AddrMode {
    Immediate,
    X,
    Direct,
    DirectX,
    DirectY,
    DirectXIndirect,
    DirectIndirectY,
    Absolute,
    AbsoluteX,
    AbsoluteY,
}

impl<'a> Context<'a> {
    pub fn read_8(&mut self, addr: u16) -> u8 {
        bus::read::<bus::DebugAccess>(self.apu, addr)
    }

    pub fn read_16(&mut self, addr: u16) -> u16 {
        self.read_8(addr) as u16 | (self.read_8(addr.wrapping_add(1)) as u16) << 8
    }

    pub fn consume_imm_8(&mut self) -> u8 {
        let res = self.read_8(self.pc);
        self.pc = self.pc.wrapping_add(1);
        res
    }

    pub fn consume_imm_16(&mut self) -> u16 {
        let res = self.read_16(self.pc);
        self.pc = self.pc.wrapping_add(2);
        res
    }

    fn resolve_direct_addr(&self, offset: u8) -> Option<u16> {
        self.direct_page_base.map(|base| base | offset as u16)
    }

    pub fn read_direct_addr(&mut self) -> u8 {
        self.consume_imm_8()
    }

    pub fn read_absolute_addr(&mut self) -> u16 {
        self.consume_imm_16()
    }

    fn read_indirect_addr(&mut self, indirect_addr: u16) -> u16 {
        self.read_16(indirect_addr)
    }

    pub fn handle_direct_addr_custom(&mut self, offset: u8, idx_opcode: &str, idx_op_addr: &str) {
        write!(self.next_instr.opcode, "${:02X}{}", offset, idx_opcode).unwrap();
        self.next_instr.op_addr = if let Some(addr) = self.resolve_direct_addr(offset) {
            format!("{:04X}{}", addr, idx_op_addr)
        } else {
            format!("P + {:02X}{}", offset, idx_op_addr)
        }
    }

    pub fn handle_direct_addr(&mut self, idx_opcode: &str, idx_op_addr: &str) {
        let offset = self.read_direct_addr();
        self.handle_direct_addr_custom(offset, idx_opcode, idx_op_addr);
    }

    fn handle_indirect_addr(
        &mut self,
        direct_idx_opcode: &str,
        direct_idx_op_addr: &str,
        indirect_idx_opcode: &str,
        indirect_idx_op_addr: &str,
    ) {
        let indirect_offset = self.read_direct_addr();
        write!(
            self.next_instr.opcode,
            "[${:02X}{}]{}",
            indirect_offset, direct_idx_opcode, indirect_idx_opcode
        )
        .unwrap();
        self.next_instr.op_addr =
            if let Some(indirect_addr) = self.resolve_direct_addr(indirect_offset) {
                if direct_idx_op_addr.is_empty() {
                    let addr = self.read_indirect_addr(indirect_addr);
                    format!("{:04X}{}", addr, indirect_idx_op_addr)
                } else {
                    format!(
                        "[{:04X}{}]{}",
                        indirect_addr, direct_idx_op_addr, indirect_idx_op_addr
                    )
                }
            } else {
                format!(
                    "[P + {:02X}{}]{}",
                    indirect_offset, direct_idx_op_addr, indirect_idx_op_addr
                )
            };
    }

    pub fn handle_absolute_addr_custom(&mut self, addr: u16, idx_opcode: &str, idx_op_addr: &str) {
        write!(self.next_instr.opcode, "!${:04X}{}", addr, idx_opcode).unwrap();
        self.next_instr.op_addr = format!("{:04X}{}", addr, idx_op_addr);
    }

    pub fn handle_absolute_addr(&mut self, idx_opcode: &str, idx_op_addr: &str) {
        let addr = self.read_absolute_addr();
        self.handle_absolute_addr_custom(addr, idx_opcode, idx_op_addr);
    }

    pub fn handle_mem_op<const ADDR: AddrMode>(&mut self) {
        match ADDR {
            AddrMode::Immediate => {
                let value = self.consume_imm_8();
                write!(self.next_instr.opcode, "#${:02X}", value).unwrap();
            }
            AddrMode::X => write!(self.next_instr.opcode, "(X)").unwrap(),
            AddrMode::Direct => self.handle_direct_addr("", ""),
            AddrMode::DirectX => self.handle_direct_addr("+X", " + X"),
            AddrMode::DirectY => self.handle_direct_addr("+Y", " + Y"),
            AddrMode::DirectXIndirect => self.handle_indirect_addr("+X", " + X", "", ""),
            AddrMode::DirectIndirectY => self.handle_indirect_addr("", "", "+Y", " + Y"),
            AddrMode::Absolute => self.handle_absolute_addr("", ""),
            AddrMode::AbsoluteX => self.handle_absolute_addr("+X", " + X"),
            AddrMode::AbsoluteY => self.handle_absolute_addr("+Y", " + Y"),
        }
    }

    pub fn handle_branch_offset(&mut self) {
        let offset = self.consume_imm_8() as i8 as i16;
        write!(
            self.next_instr.opcode,
            "${}{:02X}",
            if offset < 0 { "-" } else { "" },
            if offset < 0 { -offset } else { offset }
        )
        .unwrap();
        write!(
            self.next_instr.op_addr,
            "{:04X}",
            self.pc.wrapping_add(offset as u16)
        )
        .unwrap();
    }
}
