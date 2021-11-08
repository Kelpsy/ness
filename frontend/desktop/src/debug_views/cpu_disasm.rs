use super::{FrameDataSlot, View};
use crate::ui::window::Window;
use imgui::{ChildWindow, StyleColor};
use ness_core::{
    cpu::disasm::{disassemble_count_with_emu_state, Instr},
    emu::Emu,
};

pub struct CpuDisasm {
    start_addr_input: String,
    start_addr: u32,
    start_addr_changed: bool,
    lines: u16,
    pc: u32,
    instrs: Vec<Instr>,
}

#[derive(Clone)]
pub struct EmuState {
    start_addr: u32,
    lines: u16,
}

#[derive(Clone)]
pub struct FrameData {
    pc: u32,
    instrs: Vec<Instr>,
}

impl View for CpuDisasm {
    const NAME: &'static str = "CPU disassembly";

    type FrameData = FrameData;
    type EmuState = EmuState;

    fn new(_window: &mut Window) -> Self {
        CpuDisasm {
            start_addr_input: String::new(),
            start_addr: 0,
            start_addr_changed: true,
            lines: 32,
            pc: 0,
            instrs: Vec::new(),
        }
    }

    fn destroy(self, _window: &mut Window) {}

    fn emu_state(&self) -> Self::EmuState {
        EmuState {
            start_addr: 0,
            lines: 32,
        }
    }

    fn prepare_frame_data<'a, S: FrameDataSlot<'a, Self::FrameData>>(
        emu_state: &Self::EmuState,
        emu: &mut Emu,
        frame_data: S,
    ) {
        let frame_data = frame_data.get_or_insert_with(|| FrameData {
            pc: 0,
            instrs: Vec::new(),
        });
        frame_data.pc = emu.cpu.regs.pc as u32 | (emu.cpu.regs.code_bank() as u32) << 16;
        frame_data.instrs.clear();
        disassemble_count_with_emu_state(
            emu,
            emu_state.start_addr,
            emu_state.lines as usize,
            &mut frame_data.instrs,
        );
    }

    fn update_from_frame_data(&mut self, frame_data: &Self::FrameData, _window: &mut Window) {
        self.pc = frame_data.pc;
        self.instrs.clear();
        self.instrs.extend_from_slice(&frame_data.instrs);
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
        let mut emu_state_changed = false;

        let _mono_font = ui.push_font(window.mono_font);
        let style = ui.clone_style();
        ui.align_text_to_frame_padding();

        if ui.button("Disassemble at PC") {
            self.start_addr = self.pc;
            self.start_addr_changed = true;
        }

        ui.same_line();

        ui.set_next_item_width(ui.calc_text_size("000000")[0] + style.frame_padding[0] * 2.0);
        if ui
            .input_text("##address", &mut self.start_addr_input)
            .auto_select_all(true)
            .chars_hexadecimal(true)
            .enter_returns_true(true)
            .no_horizontal_scroll(true)
            .build()
        {
            if let Ok(addr) = u32::from_str_radix(&self.start_addr_input, 16) {
                self.start_addr = addr.clamp(0, 0xFF_FFFF);
                self.start_addr_changed = true;
            }
        }

        ui.same_line();

        let mut lines = self.lines as i32;
        ui.text("Lines:");
        ui.same_line();
        ui.set_next_item_width(
            ui.calc_text_size("0000")[0]
                + style.frame_padding[0] * 2.0
                + style.frame_padding[1] * 4.0
                + ui.current_font_size() * 2.0
                + style.item_inner_spacing[0] * 2.0,
        );
        if ui.input_int("", &mut lines).build() {
            emu_state_changed = true;
        }
        self.lines = lines.clamp(0, 0xFFFF) as u16;

        if self.start_addr_changed {
            emu_state_changed = true;
            self.start_addr_changed = false;
            self.start_addr_input = format!("{:06X}", self.start_addr);
        }

        ui.separator();

        ChildWindow::new("##instrs")
            .movable(false)
            .size([0.0, 0.0])
            .build(ui, || {
                let mut comments = String::new();
                for instr in &self.instrs {
                    comments.clear();
                    if !instr.op_addr.is_empty() {
                        comments += " ; ";
                        comments += &instr.op_addr;
                    }
                    if !instr.comment.is_empty() {
                        if comments.is_empty() {
                            comments += " ; ";
                        } else {
                            comments += ", ";
                        }
                        comments += &instr.comment;
                    }

                    ui.text(&format!("{:06X}: {}", instr.addr, instr.opcode));
                    if !comments.is_empty() {
                        ui.same_line_with_spacing(0.0, 0.0);
                        ui.text_colored(ui.style_color(StyleColor::TextDisabled), &comments);
                    }
                }
            });

        if emu_state_changed {
            Some(EmuState {
                start_addr: self.start_addr,
                lines: self.lines,
            })
        } else {
            None
        }
    }
}
