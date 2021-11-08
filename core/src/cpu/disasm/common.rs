pub use super::super::common::{AddrMode, JumpAddr, RegSize};
use super::Context;
use crate::cpu::bus;
use core::fmt::Write;

impl<'a> Context<'a> {
    pub fn read_8(&mut self, addr: u32) -> u8 {
        bus::read::<bus::DebugCpuAccess>(self.emu, addr)
    }

    pub fn read_16_bank0(&mut self, addr: u16) -> u16 {
        self.read_8(addr as u32) as u16 | (self.read_8(addr.wrapping_add(1) as u32) as u16) << 8
    }

    pub fn consume_imm<T: RegSize>(&mut self) -> T {
        if self.next_instr.followed_by_bank_boundary_crossing {
            self.next_instr.comment = "wraps around to start of bank".to_string();
        }
        let res = if T::IS_U16 {
            T::trunc_u16(
                self.read_8(self.code_bank_base | self.pc as u32) as u16
                    | (self.read_8(self.code_bank_base | self.pc.wrapping_add(1) as u32) as u16)
                        << 8,
            )
        } else {
            T::zext_u8(self.read_8(self.code_bank_base | self.pc as u32))
        };
        let (new_pc, overflowed) = self.pc.overflowing_add(T::SIZE as u16);
        self.pc = new_pc;
        self.next_instr.followed_by_bank_boundary_crossing |= overflowed;
        res
    }

    fn resolve_direct_addr(&self, offset: u8) -> Option<u16> {
        self.direct_page_offset
            .map(|dp_off| dp_off.wrapping_add(offset as u16))
    }

    fn resolve_short_addr(&self, short_addr: u16) -> Option<u32> {
        self.data_bank_base
            .map(|data_bank_base| data_bank_base | short_addr as u32)
    }

    fn read_direct_addr(&mut self) -> u8 {
        self.consume_imm()
    }

    pub fn read_absolute_short_addr(&mut self) -> u16 {
        self.consume_imm()
    }

    pub fn read_absolute_long_addr(&mut self) -> u32 {
        self.consume_imm::<u16>() as u32 | (self.consume_imm::<u8>() as u32) << 16
    }

    pub fn read_indirect_short_addr(&mut self, indirect_addr: u16) -> u16 {
        self.read_16_bank0(indirect_addr)
    }

    pub fn read_indirect_long_addr(&mut self, indirect_addr: u16) -> u32 {
        self.read_16_bank0(indirect_addr) as u32
            | (self.read_8(indirect_addr.wrapping_add(2) as u32) as u32) << 16
    }

    fn handle_direct_addr(&mut self, idx_opcode: &str, idx_op_addr: &str) {
        let offset = self.read_direct_addr();
        write!(self.next_instr.opcode, "${:02X}{}", offset, idx_opcode).unwrap();
        self.next_instr.op_addr = if let Some(addr) = self.resolve_direct_addr(offset) {
            format!("{:04X}{}", addr, idx_op_addr)
        } else {
            format!("DO + {:02X}{}", offset, idx_op_addr)
        }
    }

    fn handle_indirect_short_addr(
        &mut self,
        direct_idx_opcode: &str,
        direct_idx_op_addr: &str,
        indirect_idx_opcode: &str,
        indirect_idx_op_addr: &str,
    ) {
        let indirect_offset = self.read_direct_addr();
        write!(
            self.next_instr.opcode,
            "(${:02X}{}){}",
            indirect_offset, direct_idx_opcode, indirect_idx_opcode
        )
        .unwrap();
        self.next_instr.op_addr =
            if let Some(indirect_addr) = self.resolve_direct_addr(indirect_offset) {
                if direct_idx_op_addr.is_empty() {
                    let short_addr = self.read_indirect_short_addr(indirect_addr);
                    if let Some(addr) = self.resolve_short_addr(short_addr) {
                        format!("{:06X}{}", addr, indirect_idx_op_addr)
                    } else {
                        format!("DB + {:04X}{}", short_addr, indirect_idx_op_addr)
                    }
                } else {
                    format!(
                        "({:04X}{}){}",
                        indirect_addr, direct_idx_op_addr, indirect_idx_op_addr
                    )
                }
            } else {
                format!(
                    "(DO + {:02X}{}){}",
                    indirect_offset, direct_idx_op_addr, indirect_idx_op_addr
                )
            };
    }

    fn handle_indirect_long_addr(&mut self, idx_opcode: &str, idx_op_addr: &str) {
        let indirect_offset = self.consume_imm::<u8>();
        write!(
            self.next_instr.opcode,
            "[${:02X}]{}",
            indirect_offset, idx_opcode
        )
        .unwrap();
        self.next_instr.op_addr =
            if let Some(indirect_addr) = self.resolve_direct_addr(indirect_offset) {
                let addr = self.read_indirect_long_addr(indirect_addr);
                format!("{:06X}{}", addr, idx_op_addr)
            } else {
                format!("[DO + {:02X}]{}", indirect_offset, idx_op_addr)
            };
    }

    fn handle_absolute_short_addr(&mut self, idx_opcode: &str, idx_op_addr: &str) {
        let short_addr = self.read_absolute_short_addr();
        write!(self.next_instr.opcode, "${:04X}{}", short_addr, idx_opcode).unwrap();
        self.next_instr.op_addr = if let Some(addr) = self.resolve_short_addr(short_addr) {
            format!("{:06X}{}", addr, idx_op_addr)
        } else {
            format!("DB + {:04X}{}", short_addr, idx_op_addr)
        };
    }

    fn handle_absolute_long_addr(&mut self, idx_opcode: &str, idx_op_addr: &str) {
        let addr = self.read_absolute_long_addr();
        write!(self.next_instr.opcode, "${:06X}{}", addr, idx_opcode).unwrap();
        self.next_instr.op_addr = format!("{:06X}{}", addr, idx_op_addr);
    }

    pub fn handle_mem_op<T: RegSize, const ADDR: AddrMode>(&mut self) {
        match ADDR {
            AddrMode::Immediate => {
                let value = self.consume_imm::<T>();
                if T::IS_U16 {
                    write!(self.next_instr.opcode, "#${:04X}", value.as_zext_u16())
                } else {
                    write!(self.next_instr.opcode, "#${:02X}", value.as_trunc_u8())
                }
                .unwrap();
            }
            AddrMode::Direct => self.handle_direct_addr("", ""),
            AddrMode::DirectX => self.handle_direct_addr(",X", " + X"),
            AddrMode::DirectY => self.handle_direct_addr(",Y", " + Y"),
            AddrMode::DirectIndirect => self.handle_indirect_short_addr("", "", "", ""),
            AddrMode::DirectXIndirect => self.handle_indirect_short_addr(",X", "", " + X", ""),
            AddrMode::DirectIndirectY => self.handle_indirect_short_addr("", ",Y", "", " + Y"),
            AddrMode::DirectIndirectLong => self.handle_indirect_long_addr("", ""),
            AddrMode::DirectIndirectLongY => self.handle_indirect_long_addr(",Y", " + Y"),
            AddrMode::Absolute => self.handle_absolute_short_addr("", ""),
            AddrMode::AbsoluteX => self.handle_absolute_short_addr(",X", " + X"),
            AddrMode::AbsoluteY => self.handle_absolute_short_addr(",Y", " + Y"),
            AddrMode::AbsoluteLong => self.handle_absolute_long_addr("", ""),
            AddrMode::AbsoluteLongX => self.handle_absolute_long_addr(",X", " + X"),
            AddrMode::StackRel => {
                let offset = self.consume_imm::<u8>();
                write!(self.next_instr.opcode, "${:02X},S", offset).unwrap();
                self.next_instr.op_addr = format!("SP + {:02X}", offset);
            }
            AddrMode::StackRelIndirectY => {
                let offset = self.consume_imm::<u8>();
                write!(self.next_instr.opcode, "(${:02X},S),Y", offset).unwrap();
                self.next_instr.op_addr = format!("(SP + {:02X}) + Y", offset);
            }
        }
    }
}
