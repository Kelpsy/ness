use super::{
    common::regs::{bitfield, regs, BitfieldCommand, MaxWidth, RegCommand, RegValue},
    FrameDataSlot, View,
};
use crate::ui::window::Window;
use core::array::IntoIter;
use imgui::StyleVar;
use ness_core::{cpu::regs::Psw, emu::Emu};

#[derive(Clone, Debug)]
pub struct RegValues {
    pub a: u16,
    pub x: u16,
    pub y: u16,
    pub sp: u16,
    pub pc: u16,
    pub direct_page_offset: u16,
    pub psw: Psw,
    pub emulation_mode: bool,
    pub code_bank: u8,
    pub data_bank: u8,
}

pub struct CpuState {
    reg_values: Option<RegValues>,
}

impl View for CpuState {
    const NAME: &'static str = "CPU state";

    type FrameData = RegValues;
    type EmuState = ();

    fn new(_window: &mut Window) -> Self {
        CpuState { reg_values: None }
    }

    fn destroy(self, _window: &mut Window) {}

    fn emu_state(&self) -> Self::EmuState {}

    fn prepare_frame_data<'a, S: FrameDataSlot<'a, Self::FrameData>>(
        _emu_state: &Self::EmuState,
        emu: &mut Emu,
        frame_data: S,
    ) {
        frame_data.insert(RegValues {
            a: emu.cpu.regs.a,
            x: emu.cpu.regs.x,
            y: emu.cpu.regs.y,
            sp: emu.cpu.regs.sp,
            pc: emu.cpu.regs.pc,
            direct_page_offset: emu.cpu.regs.direct_page_offset,
            psw: emu.cpu.regs.psw(),
            emulation_mode: emu.cpu.regs.emulation_mode(),
            code_bank: emu.cpu.regs.code_bank(),
            data_bank: emu.cpu.regs.data_bank(),
        });
    }

    fn update_from_frame_data(&mut self, frame_data: &Self::FrameData, _window: &mut Window) {
        self.reg_values = Some(frame_data.clone());
    }

    fn customize_window<'a, T: AsRef<str>>(
        &mut self,
        _ui: &imgui::Ui,
        window: imgui::Window<'a, T>,
    ) -> imgui::Window<'a, T> {
        window
    }

    fn render(
        &mut self,
        ui: &imgui::Ui,
        window: &mut Window,
        _emu_running: bool,
    ) -> Option<Self::EmuState> {
        if let Some(reg_values) = self.reg_values.as_mut() {
            let _mono_font = ui.push_font(window.mono_font);
            let _frame_rounding = ui.push_style_var(StyleVar::FrameRounding(0.0));
            let _item_spacing = ui.push_style_var(StyleVar::ItemSpacing([
                0.0,
                ui.clone_style().item_spacing[1],
            ]));

            ui.columns(2, "regs", false);

            regs(
                ui,
                MaxWidth::Reg16,
                2.0,
                IntoIter::new([
                    RegCommand::Reg(
                        "A",
                        if reg_values.psw.a_is_8_bit() {
                            RegValue::Reg16Split(reg_values.a)
                        } else {
                            RegValue::Reg16(reg_values.a)
                        },
                    ),
                    RegCommand::Reg(
                        "X",
                        if reg_values.psw.index_regs_are_8_bit() {
                            RegValue::Reg8(reg_values.x as u8)
                        } else {
                            RegValue::Reg16(reg_values.x)
                        },
                    ),
                    RegCommand::Reg(
                        "Y",
                        if reg_values.psw.index_regs_are_8_bit() {
                            RegValue::Reg8(reg_values.y as u8)
                        } else {
                            RegValue::Reg16(reg_values.y)
                        },
                    ),
                    RegCommand::Callback(|ui| ui.next_column()),
                    RegCommand::Reg("SP", RegValue::Reg16(reg_values.sp)),
                    RegCommand::Reg("DO", RegValue::Reg16(reg_values.direct_page_offset)),
                    RegCommand::Reg("DB", RegValue::Reg8(reg_values.data_bank)),
                    RegCommand::Callback(|ui| {
                        ui.columns(1, "", false);
                        ui.separator();
                    }),
                    RegCommand::Reg(
                        "PC",
                        RegValue::Reg24Split(reg_values.code_bank, reg_values.pc),
                    ),
                ]),
            );

            ui.text("PSW: ");
            bitfield(
                ui,
                "PSW",
                2.0,
                reg_values.psw.0 as usize | (reg_values.emulation_mode as usize) << 8,
                &[
                    BitfieldCommand::Field("C", 1),
                    BitfieldCommand::Field("Z", 1),
                    BitfieldCommand::Field("I", 1),
                    BitfieldCommand::Field("D", 1),
                    BitfieldCommand::Field("X", 1),
                    BitfieldCommand::Field("M", 1),
                    BitfieldCommand::Field("V", 1),
                    BitfieldCommand::Field("N", 1),
                    BitfieldCommand::Callback(|ui| {
                        ui.same_line_with_spacing(0.0, 0.0);
                        ui.dummy([8.0, 0.0]);
                    }),
                    BitfieldCommand::Field("E", 1),
                ],
            );
        }
        None
    }
}
