use super::{
    common::memory::{MemoryEditor, RangeInclusive},
    FrameDataSlot, View,
};
use crate::ui::window::Window;
use ness_core::{apu::spc700::bus, emu::Emu};

pub struct SpcMemory {
    editor: MemoryEditor,
    last_visible_addrs: RangeInclusive<u64>,
    mem_contents: MemContents,
}

#[derive(Clone)]
pub struct EmuState {
    visible_addrs: RangeInclusive<u64>,
}

#[derive(Clone)]
pub struct MemContents {
    visible_addrs: RangeInclusive<u64>,
    data: Vec<u8>,
}

impl View for SpcMemory {
    const NAME: &'static str = "SPC700 memory";

    type FrameData = MemContents;
    type EmuState = EmuState;

    fn new(_window: &mut Window) -> Self {
        SpcMemory {
            editor: MemoryEditor::new()
                .show_range(false)
                .addr_range((0, 0xFFFF).into()),
            last_visible_addrs: (0, 0).into(),
            mem_contents: MemContents {
                visible_addrs: (0, 0).into(),
                data: Vec::new(),
            },
        }
    }

    fn destroy(self, _window: &mut Window) {}

    fn emu_state(&self) -> Self::EmuState {
        EmuState {
            visible_addrs: self.last_visible_addrs,
        }
    }

    fn prepare_frame_data<'a, S: FrameDataSlot<'a, Self::FrameData>>(
        emu_state: &Self::EmuState,
        emu: &mut Emu,
        frame_data: S,
    ) {
        let frame_data = frame_data.get_or_insert_with(|| MemContents {
            visible_addrs: RangeInclusive { start: 0, end: 0 },
            data: Vec::new(),
        });
        frame_data.data.clear();
        frame_data
            .data
            .reserve((emu_state.visible_addrs.end - emu_state.visible_addrs.start + 1) as usize);
        for addr in emu_state.visible_addrs {
            frame_data
                .data
                .push(bus::read::<bus::DebugAccess>(&mut emu.apu, addr as u16));
        }
        frame_data.visible_addrs = emu_state.visible_addrs;
    }

    fn update_from_frame_data(&mut self, frame_data: &Self::FrameData, _window: &mut Window) {
        self.mem_contents.data.clear();
        self.mem_contents.data.extend_from_slice(&frame_data.data);
        self.mem_contents.visible_addrs = frame_data.visible_addrs;
    }

    fn customize_window<'a, T: AsRef<str>>(
        &mut self,
        ui: &imgui::Ui,
        window: imgui::Window<'a, T>,
    ) -> imgui::Window<'a, T> {
        let width = self.editor.window_width(ui);
        window.size_constraints([width, 0.0], [width, f32::INFINITY])
    }

    fn render(
        &mut self,
        ui: &imgui::Ui,
        window: &mut Window,
        _emu_running: bool,
    ) -> Option<Self::EmuState> {
        let _mono_font = ui.push_font(window.mono_font);

        self.editor.handle_options_right_click(ui);
        self.editor.draw_callbacks(ui, None, &mut (), |_, addr| {
            if self.mem_contents.visible_addrs.contains(&addr) {
                self.mem_contents
                    .data
                    .get((addr - self.mem_contents.visible_addrs.start) as usize)
                    .copied()
            } else {
                None
            }
        });

        let visible_addrs = self.editor.visible_addrs(1);
        if visible_addrs != self.last_visible_addrs {
            self.last_visible_addrs = visible_addrs;
            Some(EmuState { visible_addrs })
        } else {
            None
        }
    }
}
