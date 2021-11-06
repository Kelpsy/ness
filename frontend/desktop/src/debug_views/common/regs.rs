use imgui::Ui;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegValue {
    Reg8(u8),
    Reg16(u16),
    Reg16Split(u16),
    Reg24Split(u8, u16),
    Reg32(u32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaxWidth {
    Reg8,
    Reg16,
    Reg24,
    Reg32,
}

pub enum RegCommand<'a> {
    Reg(&'a str, RegValue),
    Callback(fn(&Ui)),
}

pub fn regs<'a>(
    ui: &Ui,
    max_width: MaxWidth,
    spacing: f32,
    cmds: impl IntoIterator<Item = RegCommand<'a>>,
) {
    let style = ui.clone_style();
    let reg_8_bit_width = style.frame_padding[0] * 2.0 + ui.calc_text_size("00")[0];
    let reg_16_bit_width = reg_8_bit_width * 2.0 + spacing;
    let reg_24_bit_width = reg_8_bit_width + spacing + reg_16_bit_width;
    let reg_32_bit_width = reg_16_bit_width * 2.0 + spacing;
    let max_width = match max_width {
        MaxWidth::Reg8 => reg_8_bit_width,
        MaxWidth::Reg16 => reg_16_bit_width,
        MaxWidth::Reg24 => reg_24_bit_width,
        MaxWidth::Reg32 => reg_32_bit_width,
    };
    for cmd in cmds {
        match cmd {
            RegCommand::Reg(name_and_padding, value) => {
                ui.align_text_to_frame_padding();
                let name = name_and_padding.trim_end();
                ui.text(&format!("{}: {}", name, &name_and_padding[name.len()..]));
                ui.same_line();
                match value {
                    RegValue::Reg8(value) => {
                        ui.dummy([max_width - reg_8_bit_width, 0.0]);
                        ui.same_line_with_spacing(0.0, 0.0);
                        ui.set_next_item_width(reg_8_bit_width);
                        ui.input_text(&format!("##{}", name), &mut format!("{:02X}", value))
                            .read_only(true)
                            .build();
                    }
                    RegValue::Reg16(value) => {
                        ui.dummy([max_width - reg_16_bit_width, 0.0]);
                        ui.same_line_with_spacing(0.0, 0.0);
                        ui.set_next_item_width(reg_16_bit_width);
                        ui.input_text(&format!("##{}", name), &mut format!("{:04X}", value))
                            .read_only(true)
                            .build();
                    }
                    RegValue::Reg16Split(value) => {
                        ui.dummy([max_width - reg_16_bit_width, 0.0]);
                        ui.set_next_item_width(reg_8_bit_width);
                        ui.input_text(
                            &format!("##{}_high", name),
                            &mut format!("{:02X}", value >> 8),
                        )
                        .read_only(true)
                        .build();
                        ui.same_line_with_spacing(0.0, spacing);
                        ui.set_next_item_width(reg_8_bit_width);
                        ui.input_text(
                            &format!("##{}_low", name),
                            &mut format!("{:02X}", value as u8),
                        )
                        .read_only(true)
                        .build();
                    }
                    RegValue::Reg24Split(high, low) => {
                        ui.dummy([max_width - reg_24_bit_width, 0.0]);
                        ui.set_next_item_width(reg_8_bit_width);
                        ui.input_text(&format!("##{}_high", name), &mut format!("{:02X}", high))
                            .read_only(true)
                            .build();
                        ui.same_line_with_spacing(0.0, spacing);
                        ui.set_next_item_width(reg_16_bit_width);
                        ui.input_text(&format!("##{}_low", name), &mut format!("{:04X}", low))
                            .read_only(true)
                            .build();
                    }
                    RegValue::Reg32(value) => {
                        ui.dummy([max_width - reg_32_bit_width, 0.0]);
                        ui.same_line_with_spacing(0.0, 0.0);
                        ui.set_next_item_width(reg_32_bit_width);
                        ui.input_text(&format!("##{}", name), &mut format!("{:08X}", value))
                            .read_only(true)
                            .build();
                    }
                }
            }
            RegCommand::Callback(f) => f(ui),
        }
    }
}

pub enum BitfieldCommand<'a> {
    Field(&'a str, u32),
    Callback(fn(&Ui)),
    CallbackName(fn(&Ui)),
    CallbackValue(fn(&Ui)),
}

pub fn bitfield(ui: &Ui, ident: &str, spacing: f32, value: usize, cmds: &[BitfieldCommand]) {
    let mut field_widths: Vec<f32> = vec![];
    let mut total_bits = 0;
    {
        let bit_padding = ui.clone_style().frame_padding[0];
        let bit_value_width = ui.calc_text_size("0")[0];
        for cmd in cmds.iter().rev() {
            if let BitfieldCommand::Field(name, bits) = cmd {
                field_widths.push(
                    ui.calc_text_size(name)[0].max(*bits as f32 * bit_value_width)
                        + 2.0 * bit_padding,
                );
                total_bits += *bits;
            }
        }
    }

    let mut first = true;
    let mut field_i = 0;

    let mut cur_bit = total_bits;
    for cmd in cmds.iter().rev() {
        match cmd {
            BitfieldCommand::Field(_, bits) => {
                if !first {
                    ui.same_line_with_spacing(0.0, spacing);
                }
                first = false;
                cur_bit -= *bits;
                ui.set_next_item_width(field_widths[field_i]);
                ui.input_text(
                    &format!("##{}_{}", ident, field_i),
                    &mut format!("{}", value >> cur_bit & ((1 << *bits) - 1)),
                )
                .read_only(true)
                .build();
                field_i += 1;
            }
            BitfieldCommand::Callback(f) => f(ui),
            BitfieldCommand::CallbackName(f) => f(ui),
            _ => {}
        }
    }

    first = true;
    field_i = 0;
    for cmd in cmds.iter().rev() {
        match cmd {
            BitfieldCommand::Field(name, _) => {
                let width = ui.calc_text_size(name)[0];
                let padding = (field_widths[field_i] - width) * 0.5;
                field_i += 1;
                if first {
                    ui.dummy([0.0; 2]);
                    ui.same_line_with_spacing(0.0, padding);
                } else {
                    ui.same_line_with_spacing(0.0, spacing + 2.0 * padding);
                }
                first = false;
                ui.text(name);
            }
            BitfieldCommand::Callback(f) => f(ui),
            BitfieldCommand::CallbackValue(f) => f(ui),
            _ => {}
        }
    }
}
