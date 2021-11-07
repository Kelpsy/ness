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

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    output_main_cpu_instr_table();
}
