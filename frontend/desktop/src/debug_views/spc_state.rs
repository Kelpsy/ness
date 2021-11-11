use super::{
    common::regs::{bitfield, regs, BitfieldCommand, MaxWidth, RegCommand, RegValue},
    FrameDataSlot, View,
};
use crate::ui::window::Window;
use imgui::StyleVar;
use ness_core::{apu::spc700::regs::Psw, emu::Emu};

#[derive(Clone, Debug)]
pub struct RegValues {
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub sp: u8,
    pub pc: u16,
    pub psw: Psw,
}

pub struct SpcState {
    reg_values: Option<RegValues>,
}

impl View for SpcState {
    const NAME: &'static str = "SPC700 state";

    type FrameData = RegValues;
    type EmuState = ();

    fn new(_window: &mut Window) -> Self {
        SpcState { reg_values: None }
    }

    fn destroy(self, _window: &mut Window) {}

    fn emu_state(&self) -> Self::EmuState {}

    fn prepare_frame_data<'a, S: FrameDataSlot<'a, Self::FrameData>>(
        _emu_state: &Self::EmuState,
        emu: &mut Emu,
        frame_data: S,
    ) {
        frame_data.insert(RegValues {
            a: emu.apu.spc700.regs.a,
            x: emu.apu.spc700.regs.x,
            y: emu.apu.spc700.regs.y,
            sp: emu.apu.spc700.regs.sp,
            pc: emu.apu.spc700.regs.pc,
            psw: emu.apu.spc700.regs.psw(),
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
        window.always_auto_resize(true)
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
                MaxWidth::Reg8,
                2.0,
                [
                    RegCommand::Reg("A ", RegValue::Reg8(reg_values.a)),
                    RegCommand::Reg("SP", RegValue::Reg8(reg_values.sp)),
                    RegCommand::Callback(|ui| ui.next_column()),
                    RegCommand::Reg("X", RegValue::Reg8(reg_values.x)),
                    RegCommand::Reg("Y", RegValue::Reg8(reg_values.y)),
                ],
            );

            ui.columns(1, "", false);
            ui.separator();

            regs(
                ui,
                MaxWidth::Reg16,
                2.0,
                [RegCommand::Reg("PC", RegValue::Reg16(reg_values.pc))],
            );

            ui.text("PSW: ");
            bitfield(
                ui,
                "PSW",
                2.0,
                reg_values.psw.0 as usize,
                &[
                    BitfieldCommand::Field("C", 1),
                    BitfieldCommand::Field("Z", 1),
                    BitfieldCommand::Field("I", 1),
                    BitfieldCommand::Field("H", 1),
                    BitfieldCommand::Field("B", 1),
                    BitfieldCommand::Field("P", 1),
                    BitfieldCommand::Field("V", 1),
                    BitfieldCommand::Field("N", 1),
                ],
            );
        }
        None
    }
}
