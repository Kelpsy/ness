use std::{io, path::Path};

fn output_instr_table(
    filename: impl AsRef<Path>,
    table: impl IntoIterator<Item = String>,
) -> Result<(), io::Error> {
    use std::{
        env,
        fs::File,
        io::{BufWriter, Write},
    };
    let mut file = BufWriter::new(File::create(
        Path::new(&env::var_os("OUT_DIR").unwrap()).join(filename),
    )?);
    writeln!(file, "[")?;
    for (i, instr) in table.into_iter().enumerate() {
        write!(file, "{},", instr)?;
        if i & 7 == 7 {
            writeln!(file)?;
        }
    }
    writeln!(file, "]")
}

fn output_main_cpu_instr_table() {
    let instrs = (0..0x800).map(|instr| {
        let decimal_mode = instr & 1 << 8 != 0;
        let idx_ty = if instr & 1 << 9 == 0 { "u16" } else { "u8" };
        let acc_ty = if instr & 1 << 10 == 0 { "u16" } else { "u8" };
        if (instr & 1 != 0 && instr & 0xF != 0xB) || instr & 0x1F == 0x12 {
            let (op, uses_decimal_mode) = match instr >> 5 & 7 {
                0 => ("ora", false),
                1 => ("and", false),
                2 => ("eor", false),
                3 => ("adc", true),
                4 => (if instr as u8 == 0x89 { "bit" } else { "sta" }, false),
                5 => ("lda", false),
                6 => ("cmp", false),
                7 => ("sbc", true),
                _ => unreachable!(),
            };
            let addr_mode = match instr & 0x1F {
                0x01 => "DirectXIndirect",
                0x03 => "StackRel",
                0x05 => "Direct",
                0x07 => "DirectIndirectLong",
                0x09 => "Immediate",
                0x0D => "Absolute",
                0x0F => "AbsoluteLong",
                0x11 => "DirectIndirectY",
                0x12 => "DirectIndirect",
                0x13 => "StackRelIndirectY",
                0x15 => "DirectX",
                0x17 => "DirectIndirectLongY",
                0x19 => "AbsoluteY",
                0x1D => "AbsoluteX",
                0x1F => "AbsoluteLongX",
                _ => unreachable!(),
            };
            if uses_decimal_mode {
                format!(
                    "{}::<{}, {}, {{AddrMode::{}}}, {}>",
                    op, acc_ty, idx_ty, addr_mode, decimal_mode,
                )
            } else {
                format!(
                    "{}::<{}, {}, {{AddrMode::{}}}>",
                    op, acc_ty, idx_ty, addr_mode,
                )
            }
        } else if instr & 0xFD != 0x9C
            && (instr & 7 == 6
                || (instr & 7 == 4 && (instr & 0xE0 == 0x20 || instr & 0xC0 == 0x80)))
        {
            let (op, op_uses_index) = if instr & 7 == 6 {
                match instr >> 5 & 7 {
                    0 => ("asl", false),
                    1 => ("rol", false),
                    2 => ("lsr", false),
                    3 => ("ror", false),
                    4 => ("stx", true),
                    5 => ("ldx", true),
                    6 => ("dec", false),
                    7 => ("inc", false),
                    _ => unreachable!(),
                }
            } else {
                match instr >> 5 & 7 {
                    1 => ("bit", false),
                    4 => ("sty", true),
                    5 => ("ldy", true),
                    _ => unreachable!(),
                }
            };
            let addr_mode = match instr >> 3 & 3 {
                0 => "Direct",
                1 => "Absolute",
                2 => {
                    if instr & 0xC7 == 0x86 {
                        "DirectY"
                    } else {
                        "DirectX"
                    }
                }
                3 => {
                    if instr & 0xC7 == 0x86 {
                        "AbsoluteY"
                    } else {
                        "AbsoluteX"
                    }
                }
                _ => unreachable!(),
            };
            if op_uses_index {
                format!("{}::<{}, {{AddrMode::{}}}>", op, idx_ty, addr_mode)
            } else {
                format!(
                    "{}::<{}, {}, {{AddrMode::{}}}>",
                    op, acc_ty, idx_ty, addr_mode
                )
            }
        } else {
            match instr as u8 {
                0x00 => "brk".to_string(),
                0x02 => "cop".to_string(),
                0x04 => format!("tsb::<{}, {{AddrMode::Direct}}>", acc_ty),
                0x08 => "php".to_string(),
                0x0A => format!("asl_a::<{}>", acc_ty),
                0x0B => "phd".to_string(),
                0x0C => format!("tsb::<{}, {{AddrMode::Absolute}}>", acc_ty),
                0x10 => "b_cond::<7, false>".to_string(),
                0x14 => format!("trb::<{}, {{AddrMode::Direct}}>", acc_ty),
                0x18 => "clc".to_string(),
                0x1A => format!("inc_a::<{}>", acc_ty),
                0x1B => "tcs".to_string(),
                0x1C => format!("trb::<{}, {{AddrMode::Absolute}}>", acc_ty),
                0x20 => "jmp::<true, {JumpAddr::Absolute}>".to_string(),
                0x22 => "jmp::<true, {JumpAddr::AbsoluteLong}>".to_string(),
                0x28 => "plp".to_string(),
                0x2A => format!("rol_a::<{}>", acc_ty),
                0x2B => "pld".to_string(),
                0x30 => "b_cond::<7, true>".to_string(),
                0x38 => "sec".to_string(),
                0x3A => format!("dec_a::<{}>", acc_ty),
                0x3B => "tsc".to_string(),
                0x40 => "rti".to_string(),
                0x42 => "wdm".to_string(),
                0x44 => format!("mvp::<{}>", idx_ty),
                0x48 => format!("pha::<{}>", acc_ty),
                0x4A => format!("lsr_a::<{}>", acc_ty),
                0x4B => "phk".to_string(),
                0x4C => "jmp::<false, {JumpAddr::Absolute}>".to_string(),
                0x50 => "b_cond::<6, false>".to_string(),
                0x54 => format!("mvn::<{}>", idx_ty),
                0x58 => "cli".to_string(),
                0x5A => format!("phy::<{}>", idx_ty),
                0x5B => "tcd".to_string(),
                0x5C => "jmp::<false, {JumpAddr::AbsoluteLong}>".to_string(),
                0x60 => "rts".to_string(),
                0x62 => "per".to_string(),
                0x64 => format!("stz::<{}, {}, {{AddrMode::Direct}}>", acc_ty, idx_ty),
                0x68 => format!("pla::<{}>", acc_ty),
                0x6A => format!("ror_a::<{}>", acc_ty),
                0x6B => "rtl".to_string(),
                0x6C => "jmp::<false, {JumpAddr::AbsoluteIndirect}>".to_string(),
                0x70 => "b_cond::<6, true>".to_string(),
                0x74 => format!("stz::<{}, {}, {{AddrMode::DirectX}}>", acc_ty, idx_ty),
                0x78 => "sei".to_string(),
                0x7A => format!("ply::<{}>", idx_ty),
                0x7B => "tdc".to_string(),
                0x7C => "jmp::<false, {JumpAddr::AbsoluteXIndirect}>".to_string(),
                0x80 => "bra".to_string(),
                0x82 => "brl".to_string(),
                0x88 => format!("dey::<{}>", idx_ty),
                0x8A => format!("txa::<{}>", acc_ty),
                0x8B => "phb".to_string(),
                0x90 => "b_cond::<0, false>".to_string(),
                0x98 => format!("tya::<{}>", acc_ty),
                0x9A => "txs".to_string(),
                0x9B => format!("txy::<{}>", idx_ty),
                0x9C => format!("stz::<{}, {}, {{AddrMode::Absolute}}>", acc_ty, idx_ty),
                0x9E => format!("stz::<{}, {}, {{AddrMode::AbsoluteX}}>", acc_ty, idx_ty),
                0xA0 => format!("ldy::<{}, {{AddrMode::Immediate}}>", idx_ty),
                0xA2 => format!("ldx::<{}, {{AddrMode::Immediate}}>", idx_ty),
                0xA8 => format!("tay::<{}>", idx_ty),
                0xAA => format!("tax::<{}>", idx_ty),
                0xAB => "plb".to_string(),
                0xB0 => "b_cond::<0, true>".to_string(),
                0xB8 => "clv".to_string(),
                0xBA => format!("tsx::<{}>", idx_ty),
                0xBB => format!("tyx::<{}>", idx_ty),
                0xC0 => format!("cpy::<{}, {{AddrMode::Immediate}}>", idx_ty),
                0xC2 => "rep".to_string(),
                0xC4 => format!("cpy::<{}, {{AddrMode::Direct}}>", idx_ty),
                0xC8 => format!("iny::<{}>", idx_ty),
                0xCA => format!("dex::<{}>", idx_ty),
                0xCB => "wai".to_string(),
                0xCC => format!("cpy::<{}, {{AddrMode::Absolute}}>", idx_ty),
                0xD0 => "b_cond::<1, false>".to_string(),
                0xD4 => "pei".to_string(),
                0xD8 => "cld".to_string(),
                0xDA => format!("phx::<{}>", idx_ty),
                0xDB => "stp".to_string(),
                0xDC => "jmp::<false, {JumpAddr::AbsoluteIndirectLong}>".to_string(),
                0xE0 => format!("cpx::<{}, {{AddrMode::Immediate}}>", idx_ty),
                0xE2 => "sep".to_string(),
                0xE4 => format!("cpx::<{}, {{AddrMode::Direct}}>", idx_ty),
                0xE8 => format!("inx::<{}>", idx_ty),
                0xEA => "nop".to_string(),
                0xEB => "xba".to_string(),
                0xEC => format!("cpx::<{}, {{AddrMode::Absolute}}>", idx_ty),
                0xF0 => "b_cond::<1, true>".to_string(),
                0xF4 => "pea".to_string(),
                0xF8 => "sed".to_string(),
                0xFA => format!("plx::<{}>", idx_ty),
                0xFB => "xce".to_string(),
                0xFC => "jmp::<true, {JumpAddr::AbsoluteXIndirect}>".to_string(),
                _ => unreachable!(),
            }
        }
    });
    output_instr_table("instr_table_65c816.rs", instrs)
        .expect("Couldn't output 65c816 instruction table");
}

fn output_main_cpu_disasm_instr_table() {
    let instrs = (0..0x400).map(|instr| {
        let idx_ty = if instr & 1 << 8 == 0 { "u16" } else { "u8" };
        let acc_ty = if instr & 1 << 9 == 0 { "u16" } else { "u8" };
        if (instr & 1 != 0 && instr & 0xF != 0xB) || instr & 0x1F == 0x12 {
            let op = match instr >> 5 & 7 {
                0 => "ORA",
                1 => "AND",
                2 => "EOR",
                3 => "ADC",
                4 => {
                    if instr as u8 == 0x89 {
                        "BIT"
                    } else {
                        "STA"
                    }
                }
                5 => "LDA",
                6 => "CMP",
                7 => "SBC",
                _ => unreachable!(),
            };
            let addr_mode = match instr & 0x1F {
                0x01 => "DirectXIndirect",
                0x03 => "StackRel",
                0x05 => "Direct",
                0x07 => "DirectIndirectLong",
                0x09 => "Immediate",
                0x0D => "Absolute",
                0x0F => "AbsoluteLong",
                0x11 => "DirectIndirectY",
                0x12 => "DirectIndirect",
                0x13 => "StackRelIndirectY",
                0x15 => "DirectX",
                0x17 => "DirectIndirectLongY",
                0x19 => "AbsoluteY",
                0x1D => "AbsoluteX",
                0x1F => "AbsoluteLongX",
                _ => unreachable!(),
            };
            format!(
                "mem_op::<{}, \"{}\", {{AddrMode::{}}}>",
                acc_ty, op, addr_mode
            )
        } else if instr & 0xFD != 0x9C
            && (instr & 7 == 6
                || (instr & 7 == 4 && (instr & 0xE0 == 0x20 || instr & 0xC0 == 0x80)))
        {
            let (op, operand_ty) = if instr & 7 == 6 {
                match instr >> 5 & 7 {
                    0 => ("ASL", acc_ty),
                    1 => ("ROL", acc_ty),
                    2 => ("LSR", acc_ty),
                    3 => ("ROR", acc_ty),
                    4 => ("STX", idx_ty),
                    5 => ("LDX", idx_ty),
                    6 => ("DEC", acc_ty),
                    7 => ("INC", acc_ty),
                    _ => unreachable!(),
                }
            } else {
                match instr >> 5 & 7 {
                    1 => ("BIT", acc_ty),
                    4 => ("STY", idx_ty),
                    5 => ("LDY", idx_ty),
                    _ => unreachable!(),
                }
            };
            let addr_mode = match instr >> 3 & 3 {
                0 => "Direct",
                1 => "Absolute",
                2 => {
                    if instr & 0xC7 == 0x86 {
                        "DirectY"
                    } else {
                        "DirectX"
                    }
                }
                3 => {
                    if instr & 0xC7 == 0x86 {
                        "AbsoluteY"
                    } else {
                        "AbsoluteX"
                    }
                }
                _ => unreachable!(),
            };
            format!(
                "mem_op::<{}, \"{}\", {{AddrMode::{}}}>",
                operand_ty, op, addr_mode
            )
        } else {
            match instr as u8 {
                0x00 => "marker_byte::<\"BRK\">".to_string(),
                0x02 => "marker_byte::<\"COP\">".to_string(),
                0x04 => format!("mem_op::<{}, \"TSB\", {{AddrMode::Direct}}>", acc_ty),
                0x08 => "raw::<\"PHP\">".to_string(),
                0x0A => "raw::<\"ASL A\">".to_string(),
                0x0B => "raw::<\"PHD\">".to_string(),
                0x0C => format!("mem_op::<{}, \"TSB\", {{AddrMode::Absolute}}>", acc_ty),
                0x10 => "branch::<\"PL\">".to_string(),
                0x14 => format!("mem_op::<{}, \"TRB\", {{AddrMode::Direct}}>", acc_ty),
                0x18 => "raw::<\"CLC\">".to_string(),
                0x1A => "raw::<\"INC A\">".to_string(),
                0x1B => "raw::<\"TCS\">".to_string(),
                0x1C => format!("mem_op::<{}, \"TRB\", {{AddrMode::Absolute}}>", acc_ty),
                0x20 => "jmp::<true, {JumpAddr::Absolute}>".to_string(),
                0x22 => "jmp::<true, {JumpAddr::AbsoluteLong}>".to_string(),
                0x28 => "raw::<\"PLP\">".to_string(),
                0x2A => "raw::<\"ROL A\">".to_string(),
                0x2B => "raw::<\"PLD\">".to_string(),
                0x30 => "branch::<\"MI\">".to_string(),
                0x38 => "raw::<\"SEC\">".to_string(),
                0x3A => "raw::<\"DEC A\">".to_string(),
                0x3B => "raw::<\"TSC\">".to_string(),
                0x40 => "raw::<\"RTI\">".to_string(),
                0x42 => "marker_byte::<\"WDM\">".to_string(),
                0x44 => "move_block::<false>".to_string(),
                0x48 => "raw::<\"PHA\">".to_string(),
                0x4A => "raw::<\"LSR A\">".to_string(),
                0x4B => "raw::<\"PHK\">".to_string(),
                0x4C => "jmp::<false, {JumpAddr::Absolute}>".to_string(),
                0x50 => "branch::<\"VC\">".to_string(),
                0x54 => "move_block::<true>".to_string(),
                0x58 => "raw::<\"CLI\">".to_string(),
                0x5A => "raw::<\"PHY\">".to_string(),
                0x5B => "raw::<\"TCD\">".to_string(),
                0x5C => "jmp::<false, {JumpAddr::AbsoluteLong}>".to_string(),
                0x60 => "raw::<\"RTS\">".to_string(),
                0x62 => "per".to_string(),
                0x64 => format!("mem_op::<{}, \"STZ\", {{AddrMode::Direct}}>", acc_ty),
                0x68 => "raw::<\"PLA\">".to_string(),
                0x6A => "raw::<\"ROR A\">".to_string(),
                0x6B => "raw::<\"RTL\">".to_string(),
                0x6C => "jmp::<false, {JumpAddr::AbsoluteIndirect}>".to_string(),
                0x70 => "branch::<\"VS\">".to_string(),
                0x74 => format!("mem_op::<{}, \"STZ\", {{AddrMode::DirectX}}>", acc_ty),
                0x78 => "raw::<\"SEI\">".to_string(),
                0x7A => "raw::<\"PLY\">".to_string(),
                0x7B => "raw::<\"TDC\">".to_string(),
                0x7C => "jmp::<false, {JumpAddr::AbsoluteXIndirect}>".to_string(),
                0x80 => "branch::<\"RA\">".to_string(),
                0x82 => "brl".to_string(),
                0x88 => "raw::<\"DEY\">".to_string(),
                0x8A => "raw::<\"TXA\">".to_string(),
                0x8B => "raw::<\"PHB\">".to_string(),
                0x90 => "branch::<\"CC\">".to_string(),
                0x98 => "raw::<\"TYA\">".to_string(),
                0x9A => "raw::<\"TXS\">".to_string(),
                0x9B => "raw::<\"TXY\">".to_string(),
                0x9C => format!("mem_op::<{}, \"STZ\", {{AddrMode::Absolute}}>", acc_ty),
                0x9E => format!("mem_op::<{}, \"STZ\", {{AddrMode::AbsoluteX}}>", acc_ty),
                0xA0 => format!("mem_op::<{}, \"LDY\", {{AddrMode::Immediate}}>", idx_ty),
                0xA2 => format!("mem_op::<{}, \"LDX\", {{AddrMode::Immediate}}>", idx_ty),
                0xA8 => "raw::<\"TAY\">".to_string(),
                0xAA => "raw::<\"TAX\">".to_string(),
                0xAB => "raw::<\"PLB\">".to_string(),
                0xB0 => "branch::<\"CS\">".to_string(),
                0xB8 => "raw::<\"CLV\">".to_string(),
                0xBA => "raw::<\"TSX\">".to_string(),
                0xBB => "raw::<\"TYX\">".to_string(),
                0xC0 => format!("mem_op::<{}, \"CPY\", {{AddrMode::Immediate}}>", idx_ty),
                0xC2 => "rep".to_string(),
                0xC4 => format!("mem_op::<{}, \"CPY\", {{AddrMode::Direct}}>", idx_ty),
                0xC8 => "raw::<\"INY\">".to_string(),
                0xCA => "raw::<\"DEX\">".to_string(),
                0xCB => "raw::<\"WAI\">".to_string(),
                0xCC => format!("mem_op::<{}, \"CPY\", {{AddrMode::Absolute}}>", idx_ty),
                0xD0 => "branch::<\"NE\">".to_string(),
                0xD4 => "mem_op::<u8, \"PEI\", {AddrMode::DirectIndirect}>".to_string(),
                0xD8 => "raw::<\"CLD\">".to_string(),
                0xDA => "raw::<\"PHX\">".to_string(),
                0xDB => "raw::<\"STP\">".to_string(),
                0xDC => "jmp::<false, {JumpAddr::AbsoluteIndirectLong}>".to_string(),
                0xE0 => format!("mem_op::<{}, \"CPX\", {{AddrMode::Immediate}}>", idx_ty),
                0xE2 => "sep".to_string(),
                0xE4 => format!("mem_op::<{}, \"CPX\", {{AddrMode::Direct}}>", idx_ty),
                0xE8 => "raw::<\"INX\">".to_string(),
                0xEA => "raw::<\"NOP\">".to_string(),
                0xEB => "raw::<\"XBA\">".to_string(),
                0xEC => format!("mem_op::<{}, \"CPX\", {{AddrMode::Absolute}}>", idx_ty),
                0xF0 => "branch::<\"EQ\">".to_string(),
                0xF4 => "mem_op::<u8, \"PEA\", {AddrMode::Absolute}>".to_string(),
                0xF8 => "raw::<\"SED\">".to_string(),
                0xFA => "raw::<\"PLX\">".to_string(),
                0xFB => "raw::<\"XCE\">".to_string(),
                0xFC => "jmp::<true, {JumpAddr::AbsoluteXIndirect}>".to_string(),
                _ => unreachable!(),
            }
        }
    });
    output_instr_table("instr_table_65c816_disasm.rs", instrs)
        .expect("Couldn't output 65c816 disassembly instruction table");
}

fn output_spc700_instr_table() {
    let instrs = (0..0x100).map(|instr| match instr & 0xF {
        0 => match instr >> 4 {
            0x0 => "nop",
            0x1 => "b_cond::<7, false>",
            0x2 => "set_direct_page::<false>",
            0x3 => "b_cond::<7, true>",
            0x4 => "set_direct_page::<true>",
            0x5 => "b_cond::<6, false>",
            0x6 => "set_carry::<false>",
            0x7 => "b_cond::<6, true>",
            0x8 => "set_carry::<true>",
            0x9 => "b_cond::<0, false>",
            0xA => "set_irqs_enabled::<true>",
            0xB => "b_cond::<0, true>",
            0xC => "set_irqs_enabled::<false>",
            0xD => "b_cond::<1, false>",
            0xE => "clrv",
            0xF => "b_cond::<1, true>",
            _ => unreachable!(),
        }
        .to_string(),
        1 => format!("tcall::<{}>", instr >> 4),
        2 => format!("modify_bit::<{}, {}>", instr & 0x10 == 0, instr >> 5,),
        3 => format!("branch_bit::<{}, {}>", instr & 0x10 == 0, instr >> 5,),
        4..=9 => {
            let (first_op, second_op) = match instr & 0x1F {
                0x04 => ("Reg(Reg::A)", "Direct"),
                0x05 => ("Reg(Reg::A)", "Absolute"),
                0x06 => ("Reg(Reg::A)", "X"),
                0x07 => ("Reg(Reg::A)", "DirectXIndirect"),
                0x08 => ("Reg(Reg::A)", "Immediate"),
                0x09 => ("Mem(AddrMode::Direct)", "Direct"),
                0x14 => ("Reg(Reg::A)", "DirectX"),
                0x15 => ("Reg(Reg::A)", "AbsoluteX"),
                0x16 => ("Reg(Reg::A)", "AbsoluteY"),
                0x17 => ("Reg(Reg::A)", "DirectIndirectY"),
                0x18 => ("Mem(AddrMode::Direct)", "Immediate"),
                0x19 => ("Mem(AddrMode::X)", "Y"),
                _ => unreachable!(),
            };
            let opcode = match instr >> 5 {
                0 => "or",
                1 => "and",
                2 => "eor",
                3 => "cmp",
                4 => "adc",
                5 => "sbc",
                6 => {
                    if instr & 0xE == 0x8 {
                        return match instr & 0x11 {
                            0 => "cmp::<{MemOrReg::Reg(Reg::X)}, {AddrMode::Immediate}>",
                            1 => "mov_mem_reg::<{AddrMode::Absolute}, {Reg::X}>",
                            0x10 => "mov_mem_reg::<{AddrMode::Direct}, {Reg::X}>",
                            _ => "mov_mem_reg::<{AddrMode::DirectY}, {Reg::X}>",
                        }
                        .to_string();
                    }
                    return format!("mov_mem_reg::<{{AddrMode::{}}}, {{Reg::A}}>", second_op);
                }
                7 => {
                    if instr & 0xE == 0x8 && instr != 0xE8 {
                        return match instr & 0x11 {
                            1 => "mov_reg_op::<{Reg::X}, {MemOrReg::Mem(AddrMode::Absolute)}>",
                            0x10 => "mov_reg_op::<{Reg::X}, {MemOrReg::Mem(AddrMode::Direct)}>",
                            _ => "mov_reg_op::<{Reg::X}, {MemOrReg::Mem(AddrMode::DirectY)}>",
                        }
                        .to_string();
                    }
                    return format!(
                        "mov_reg_op::<{{Reg::A}}, {{MemOrReg::Mem(AddrMode::{})}}>",
                        second_op
                    );
                }
                _ => unreachable!(),
            };
            format!(
                "{}::<{{MemOrReg::{}}}, {{AddrMode::{}}}>",
                opcode, first_op, second_op
            )
        }
        0xA => match instr >> 4 {
            0x0 => "or1::<false>",
            0x1 => "decw",
            0x2 => "or1::<true>",
            0x3 => "incw",
            0x4 => "and1::<false>",
            0x5 => "cmpw",
            0x6 => "and1::<true>",
            0x7 => "addw",
            0x8 => "eor1",
            0x9 => "subw",
            0xA => "mov1_c_mem",
            0xB => "movw_ya_direct",
            0xC => "mov1_mem_c",
            0xD => "movw_direct_ya",
            0xE => "not1",
            0xF => "mov_direct",
            _ => unreachable!(),
        }
        .to_string(),
        0xB..=0xC => {
            let (operand_mem_or_reg, operand) = match instr & 0x1F {
                0xB => ("Mem", "AddrMode::Direct"),
                0xC => ("Mem", "AddrMode::Absolute"),
                0x1B => ("Mem", "AddrMode::DirectX"),
                0x1C => ("Reg", "Reg::A"),
                _ => unreachable!(),
            };
            let opcode = match instr >> 5 {
                0 => "asl",
                1 => "rol",
                2 => "lsr",
                3 => "ror",
                4 => "dec",
                5 => "inc",
                6 => {
                    if instr == 0xDC {
                        return "dec::<{MemOrReg::Reg(Reg::Y)}>".to_string();
                    }
                    return format!("mov_mem_reg::<{{{}}}, {{Reg::Y}}>", operand);
                }
                7 => {
                    if instr == 0xFC {
                        return "inc::<{MemOrReg::Reg(Reg::Y)}>".to_string();
                    }
                    return format!("mov_reg_op::<{{Reg::Y}}, {{MemOrReg::Mem({})}}>", operand);
                }
                _ => unreachable!(),
            };
            format!(
                "{}::<{{MemOrReg::{}({})}}>",
                opcode, operand_mem_or_reg, operand
            )
        }
        _ => match instr {
            0x0D => "push_psw",
            0x1D => "dec::<{MemOrReg::Reg(Reg::X)}>",
            0x2D => "push_reg::<{Reg::A}>",
            0x3D => "inc::<{MemOrReg::Reg(Reg::X)}>",
            0x4D => "push_reg::<{Reg::X}>",
            0x5D => "mov_reg_op::<{Reg::X}, {MemOrReg::Reg(Reg::A)}>",
            0x6D => "push_reg::<{Reg::Y}>",
            0x7D => "mov_reg_op::<{Reg::A}, {MemOrReg::Reg(Reg::X)}>",
            0x8D => "mov_reg_op::<{Reg::Y}, {MemOrReg::Mem(AddrMode::Immediate)}>",
            0x9D => "mov_x_sp",
            0xAD => "cmp::<{MemOrReg::Reg(Reg::Y)}, {AddrMode::Immediate}>",
            0xBD => "mov_sp_x",
            0xCD => "mov_reg_op::<{Reg::X}, {MemOrReg::Mem(AddrMode::Immediate)}>",
            0xDD => "mov_reg_op::<{Reg::A}, {MemOrReg::Reg(Reg::Y)}>",
            0xED => "notc",
            0xFD => "mov_reg_op::<{Reg::Y}, {MemOrReg::Reg(Reg::A)}>",
            0x0E => "test_modify_bit::<true>",
            0x1E => "cmp::<{MemOrReg::Reg(Reg::X)}, {AddrMode::Absolute}>",
            0x2E => "cbne_direct",
            0x3E => "cmp::<{MemOrReg::Reg(Reg::X)}, {AddrMode::Direct}>",
            0x4E => "test_modify_bit::<false>",
            0x5E => "cmp::<{MemOrReg::Reg(Reg::Y)}, {AddrMode::Absolute}>",
            0x6E => "dbnz_direct",
            0x7E => "cmp::<{MemOrReg::Reg(Reg::Y)}, {AddrMode::Direct}>",
            0x8E => "pop_psw",
            0x9E => "div",
            0xAE => "pop_reg::<{Reg::A}>",
            0xBE => "das",
            0xCE => "pop_reg::<{Reg::X}>",
            0xDE => "cbne_direct_x",
            0xEE => "pop_reg::<{Reg::Y}>",
            0xFE => "dbnz_y",
            0x0F => "brk",
            0x1F => "jmp_abs_x_indirect",
            0x2F => "bra",
            0x3F => "call",
            0x4F => "pcall",
            0x5F => "jmp_absolute",
            0x6F => "ret",
            0x7F => "reti",
            0x8F => "mov_direct_imm",
            0x9F => "xcn",
            0xAF => "mov_mem_x_inc_a",
            0xBF => "mov_a_mem_x_inc",
            0xCF => "mul",
            0xDF => "daa",
            0xEF => "sleep",
            0xFF => "stop",
            _ => unreachable!(),
        }
        .to_string(),
    });
    output_instr_table("instr_table_spc700.rs", instrs)
        .expect("Couldn't output SPC700 instruction table");
}

fn output_spc700_disasm_instr_table() {
    let instrs = (0..0x100).map(|instr| match instr & 0xF {
        0 => match instr >> 4 {
            0x0 => "raw::<\"NOP\">",
            0x1 => "branch::<\"PL\">",
            0x2 => "set_direct_page::<false>",
            0x3 => "branch::<\"MI\">",
            0x4 => "set_direct_page::<true>",
            0x5 => "branch::<\"VC\">",
            0x6 => "raw::<\"CLRC\">",
            0x7 => "branch::<\"VS\">",
            0x8 => "raw::<\"SETC\">",
            0x9 => "branch::<\"CC\">",
            0xA => "raw::<\"EI\">",
            0xB => "branch::<\"CS\">",
            0xC => "raw::<\"DI\">",
            0xD => "branch::<\"NE\">",
            0xE => "raw::<\"CLRV\">",
            0xF => "branch::<\"EQ\">",
            _ => unreachable!(),
        }
        .to_string(),
        1 => format!("tcall::<{}>", instr >> 4),
        2 => format!(
            "modify_bit::<\"{}\", {}>",
            if instr & 0x10 == 0 { "SET1" } else { "CLR1" },
            instr >> 5,
        ),
        3 => format!("branch_bit::<{}, {}>", instr & 0x10 == 0, instr >> 5,),
        4..=9 => {
            let (opcode, a_first) = match instr >> 5 {
                0 => ("OR", true),
                1 => ("AND", true),
                2 => ("EOR", true),
                3 => ("CMP", true),
                4 => ("ADC", true),
                5 => ("SBC", true),
                6 => {
                    if instr & 0xE == 0x8 {
                        return match instr & 0x11 {
                            0 => "op_reg_mem::<\"CMP\", \"X\", {AddrMode::Immediate}>",
                            1 => "op_mem_reg::<\"MOV\", {AddrMode::Absolute}, \"X\">",
                            0x10 => "op_mem_reg::<\"MOV\", {AddrMode::Direct}, \"X\">",
                            _ => "op_mem_reg::<\"MOV\", {AddrMode::DirectY}, \"X\">",
                        }
                        .to_string();
                    }
                    ("MOV", false)
                }
                7 => {
                    if instr & 0xE == 0x8 && instr != 0xE8 {
                        return match instr & 0x11 {
                            1 => "op_reg_mem::<\"MOV\", \"X\", {AddrMode::Absolute}>",
                            0x10 => "op_reg_mem::<\"MOV\", \"X\", {AddrMode::Direct}>",
                            _ => "op_reg_mem::<\"MOV\", \"X\", {AddrMode::DirectY}>",
                        }
                        .to_string();
                    }
                    ("MOV", true)
                }
                _ => unreachable!(),
            };
            let addr_mode = match instr & 0x1F {
                0x04 => "Direct",
                0x05 => "Absolute",
                0x06 => "X",
                0x07 => "DirectXIndirect",
                0x08 => "Immediate",
                0x09 => return format!("op_direct::<\"{}\">", opcode),
                0x14 => "DirectX",
                0x15 => "AbsoluteX",
                0x16 => "AbsoluteY",
                0x17 => "DirectIndirectY",
                0x18 => return format!("op_direct_imm::<\"{}\">", opcode),
                0x19 => return format!("raw::<\"{} (X), (Y)\">", opcode),
                _ => unreachable!(),
            };
            if a_first {
                format!(
                    "op_reg_mem::<\"{}\", \"A\", {{AddrMode::{}}}>",
                    opcode, addr_mode,
                )
            } else {
                format!(
                    "op_mem_reg::<\"{}\", {{AddrMode::{}}}, \"A\">",
                    opcode, addr_mode,
                )
            }
        }
        0xA => match instr >> 4 {
            0x0 => "op_carry_mem::<\"OR1\", false>",
            0x1 => "modify_direct_word::<\"DEC\">",
            0x2 => "op_carry_mem::<\"OR1\", true>",
            0x3 => "modify_direct_word::<\"INC\">",
            0x4 => "op_carry_mem::<\"AND1\", false>",
            0x5 => "ya_mem_op::<\"CMP\">",
            0x6 => "op_carry_mem::<\"AND1\", true>",
            0x7 => "ya_mem_op::<\"ADD\">",
            0x8 => "op_carry_mem::<\"EOR1\", false>",
            0x9 => "ya_mem_op::<\"SUB\">",
            0xA => "op_carry_mem::<\"MOV1\", false>",
            0xB => "ya_mem_op::<\"MOV\">",
            0xC => "mov1_mem_carry",
            0xD => "movw_mem_ya",
            0xE => "not1",
            0xF => "op_direct::<\"MOV\">",
            _ => unreachable!(),
        }
        .to_string(),
        0xB..=0xC => {
            let addr_mode = match instr & 0x1F {
                0xB => "Direct",
                0xC => "Absolute",
                0x1B => "DirectX",
                0x1C => "",
                _ => unreachable!(),
            };
            let opcode = match instr >> 5 {
                0 => "ASL",
                1 => "ROL",
                2 => "LSR",
                3 => "ROR",
                4 => "DEC",
                5 => "INC",
                6 => {
                    if instr == 0xDC {
                        return "raw::<\"DEC Y\">".to_string();
                    }
                    return format!("op_mem_reg::<\"MOV\", {{AddrMode::{}}}, \"Y\">", addr_mode);
                }
                7 => {
                    if instr == 0xFC {
                        return "raw::<\"INC Y\">".to_string();
                    }
                    return format!("op_reg_mem::<\"MOV\", \"Y\", {{AddrMode::{}}}>", addr_mode);
                }
                _ => unreachable!(),
            };
            if instr & 0x1F == 0x1C {
                format!("raw::<\"{} A\">", opcode)
            } else {
                format!("op_mem::<\"{}\", {{AddrMode::{}}}>", opcode, addr_mode,)
            }
        }
        _ => match instr {
            0x0D => "raw::<\"PUSH PSW\">",
            0x1D => "raw::<\"DEC X\">",
            0x2D => "raw::<\"PUSH A\">",
            0x3D => "raw::<\"INC X\">",
            0x4D => "raw::<\"PUSH X\">",
            0x5D => "raw::<\"MOV X, A\">",
            0x6D => "raw::<\"PUSH Y\">",
            0x7D => "raw::<\"MOV A, X\">",
            0x8D => "op_reg_mem::<\"MOV\", \"Y\", {AddrMode::Immediate}>",
            0x9D => "raw::<\"MOV X, SP\">",
            0xAD => "op_reg_mem::<\"CMP\", \"Y\", {AddrMode::Immediate}>",
            0xBD => "raw::<\"MOV SP, X\">",
            0xCD => "op_reg_mem::<\"MOV\", \"X\", {AddrMode::Immediate}>",
            0xDD => "raw::<\"MOV A, Y\">",
            0xED => "raw::<\"NOTC\">",
            0xFD => "raw::<\"MOV Y, A\">",
            0x0E => "test_modify_bit::<true>",
            0x1E => "op_reg_mem::<\"CMP\", \"X\", {AddrMode::Absolute}>",
            0x2E => "cbne::<{AddrMode::Direct}>",
            0x3E => "op_reg_mem::<\"CMP\", \"X\", {AddrMode::Direct}>",
            0x4E => "test_modify_bit::<false>",
            0x5E => "op_reg_mem::<\"CMP\", \"Y\", {AddrMode::Absolute}>",
            0x6E => "dbnz_direct",
            0x7E => "op_reg_mem::<\"CMP\", \"Y\", {AddrMode::Direct}>",
            0x8E => "raw::<\"POP PSW\">",
            0x9E => "raw::<\"DIV YA, X\">",
            0xAE => "raw::<\"POP A\">",
            0xBE => "raw::<\"DAS\">",
            0xCE => "raw::<\"POP X\">",
            0xDE => "cbne::<{AddrMode::DirectX}>",
            0xEE => "raw::<\"POP Y\">",
            0xFE => "dbnz_y",
            0x0F => "raw::<\"BRK\">",
            0x1F => "jmp_abs_x_indirect",
            0x2F => "branch::<\"RA\">",
            0x3F => "call",
            0x4F => "pcall",
            0x5F => "jmp_absolute",
            0x6F => "raw::<\"RET\">",
            0x7F => "raw::<\"RETI\">",
            0x8F => "op_direct_imm::<\"MOV\">",
            0x9F => "raw::<\"XCN A\">",
            0xAF => "raw::<\"MOV (X)+, A\">",
            0xBF => "raw::<\"MOV A, (X)+\">",
            0xCF => "raw::<\"MUL YA\">",
            0xDF => "raw::<\"DAA\">",
            0xEF => "raw::<\"SLEEP\">",
            0xFF => "raw::<\"STOP\">",
            _ => unreachable!(),
        }
        .to_string(),
    });
    output_instr_table("instr_table_spc700_disasm.rs", instrs)
        .expect("Couldn't output SPC700 disassembly instruction table");
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    output_main_cpu_instr_table();
    output_main_cpu_disasm_instr_table();
    output_spc700_instr_table();
    output_spc700_disasm_instr_table();
}
