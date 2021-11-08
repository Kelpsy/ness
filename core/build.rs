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
                0x10 => "b_cond::<\"PL\">".to_string(),
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
                0x30 => "b_cond::<\"MI\">".to_string(),
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
                0x50 => "b_cond::<\"VC\">".to_string(),
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
                0x70 => "b_cond::<\"VS\">".to_string(),
                0x74 => format!("mem_op::<{}, \"STZ\", {{AddrMode::DirectX}}>", acc_ty),
                0x78 => "raw::<\"SEI\">".to_string(),
                0x7A => "raw::<\"PLY\">".to_string(),
                0x7B => "raw::<\"TDC\">".to_string(),
                0x7C => "jmp::<false, {JumpAddr::AbsoluteXIndirect}>".to_string(),
                0x80 => "b_cond::<\"RA\">".to_string(),
                0x82 => "brl".to_string(),
                0x88 => "raw::<\"DEY\">".to_string(),
                0x8A => "raw::<\"TXA\">".to_string(),
                0x8B => "raw::<\"PHB\">".to_string(),
                0x90 => "b_cond::<\"CC\">".to_string(),
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
                0xB0 => "b_cond::<\"CS\">".to_string(),
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
                0xD0 => "b_cond::<\"NE\">".to_string(),
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
                0xF0 => "b_cond::<\"EQ\">".to_string(),
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

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    output_main_cpu_instr_table();
    output_main_cpu_disasm_instr_table();
}
