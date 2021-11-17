#[allow(dead_code)]
mod common;
mod cpu_state;
pub use cpu_state::CpuState;
mod cpu_memory;
pub use cpu_memory::CpuMemory;
mod cpu_disasm;
pub use cpu_disasm::CpuDisasm;
mod spc_state;
pub use spc_state::SpcState;
mod spc_memory;
pub use spc_memory::SpcMemory;
mod spc_disasm;
pub use spc_disasm::SpcDisasm;

use super::ui::window::Window;
use fxhash::FxHashMap;
use imgui::MenuItem;
use ness_core::emu::Emu;
use std::collections::hash_map::Entry;

pub type ViewKey = u32;

pub trait FrameDataSlot<'a, T> {
    fn insert(self, value: T);
    fn get_or_insert_with(self, f: impl FnOnce() -> T) -> &'a mut T;
}

impl<'a, T> FrameDataSlot<'a, T> for Entry<'a, ViewKey, T> {
    fn insert(self, value: T) {
        match self {
            Entry::Occupied(mut entry) => {
                entry.insert(value);
            }
            Entry::Vacant(entry) => {
                entry.insert(value);
            }
        }
    }
    fn get_or_insert_with(self, f: impl FnOnce() -> T) -> &'a mut T {
        self.or_insert_with(f)
    }
}

impl<'a, T> FrameDataSlot<'a, T> for &'a mut Option<T> {
    fn insert(self, value: T) {
        *self = Some(value);
    }
    fn get_or_insert_with(self, f: impl FnOnce() -> T) -> &'a mut T {
        Option::get_or_insert_with(self, f)
    }
}

pub trait View {
    const NAME: &'static str;

    type FrameData;
    type EmuState: Clone;

    fn new(window: &mut Window) -> Self;
    fn destroy(self, window: &mut Window);

    fn emu_state(&self) -> Self::EmuState;
    fn prepare_frame_data<'a, S: FrameDataSlot<'a, Self::FrameData>>(
        emu_state: &Self::EmuState,
        emu: &mut Emu,
        frame_data: S,
    );

    fn update_from_frame_data(&mut self, frame_data: &Self::FrameData, window: &mut Window);
    fn customize_window<'a, T: AsRef<str>>(
        &mut self,
        ui: &imgui::Ui,
        window: imgui::Window<'a, T>,
    ) -> imgui::Window<'a, T>;
    fn render(
        &mut self,
        ui: &imgui::Ui,
        window: &mut Window,
        emu_running: bool,
    ) -> Option<Self::EmuState>;
}

macro_rules! declare_structs {
    (
        $(
            singleton
            $s_view_ident: ident,
            $s_view_ty: ty,
            $s_toggle_updates_message_ident: ident,
            $s_update_emu_state_message_ident: ident
        );*$(;)?
        $(
            instanceable
            $i_view_ident: ident,
            $i_view_ty: ty,
            $i_toggle_updates_message_ident: ident,
            $i_update_emu_state_message_ident: ident
        );*$(;)?
    ) => {
        pub enum Message {
            $(
                $s_toggle_updates_message_ident(bool),
                $s_update_emu_state_message_ident(Option<(<$s_view_ty as View>::EmuState, bool)>),
            )*
            $(
                $i_toggle_updates_message_ident(ViewKey, bool),
                $i_update_emu_state_message_ident(
                    ViewKey,
                    Option<(<$i_view_ty as View>::EmuState, bool)>,
                ),
            )*
        }

        #[derive(Clone)]
        pub struct EmuState {
            $(
                $s_view_ident: Option<(<$s_view_ty as View>::EmuState, bool)>,
            )*
            $(
                $i_view_ident: FxHashMap<ViewKey, (<$i_view_ty as View>::EmuState, bool)>,
            )*
        }

        impl EmuState {
            pub fn new() -> Self {
                EmuState {
                    $(
                        $s_view_ident: None,
                    )*
                    $(
                        $i_view_ident: FxHashMap::default(),
                    )*
                }
            }

            pub fn handle_message(&mut self, message: Message) {
                match message {
                    $(
                        Message::$s_toggle_updates_message_ident(enabled) => {
                            if let Some((_, view_enabled)) = &mut self.$s_view_ident {
                                *view_enabled = enabled;
                            }
                        }
                        Message::$s_update_emu_state_message_ident(emu_state) => {
                            self.$s_view_ident = emu_state;
                        }
                    )*
                    $(
                        Message::$i_toggle_updates_message_ident(key, enabled) => {
                            if let Some((_, view_enabled)) = self.$i_view_ident.get_mut(&key) {
                                *view_enabled = enabled;
                            }
                        }
                        Message::$i_update_emu_state_message_ident(key, emu_state) => {
                            if let Some(emu_state) = emu_state {
                                self.$i_view_ident.insert(key, emu_state);
                            } else {
                                self.$i_view_ident.remove(&key);
                            }
                        }
                    )*
                }
            }

            pub fn prepare_frame_data(
                &mut self,
                emu: &mut Emu,
                frame_data: &mut FrameData,
            ) {
                $(
                    if let Some((emu_state, visible)) = &self.$s_view_ident {
                        if *visible {
                            <$s_view_ty>::prepare_frame_data(
                                emu_state,
                                emu,
                                &mut frame_data.$s_view_ident,
                            );
                        }
                    } else {
                        frame_data.$s_view_ident = None;
                    }
                )*
                $(
                    frame_data.$i_view_ident.retain(|key, _| self.$i_view_ident.contains_key(key));
                    for (key, (emu_state, visible)) in &self.$i_view_ident {
                        if !*visible {
                            continue;
                        }
                        <$i_view_ty>::prepare_frame_data(
                            emu_state,
                            emu,
                            frame_data.$i_view_ident.entry(*key),
                        );
                    }
                )*
            }
        }

        pub struct FrameData {
            $(
                $s_view_ident: Option<<$s_view_ty as View>::FrameData>,
            )*
            $(
                $i_view_ident: FxHashMap<ViewKey, <$i_view_ty as View>::FrameData>,
            )*
        }

        impl FrameData {
            #[inline]
            #[must_use]
            pub fn new() -> Self {
                FrameData {
                    $(
                        $s_view_ident: None,
                    )*
                    $(
                        $i_view_ident: FxHashMap::default(),
                    )*
                }
            }
        }

        pub struct UiState {
            messages: Vec<Message>,
            $(
                $s_view_ident: Option<($s_view_ty, bool)>,
            )*
            $(
                $i_view_ident: (FxHashMap<ViewKey, ($i_view_ty, bool)>, ViewKey),
            )*
        }

        impl UiState {
            #[inline]
            #[must_use]
            pub fn new() -> Self {
                UiState {
                    messages: Vec::new(),
                    $(
                        $s_view_ident: None,
                    )*
                    $(
                        $i_view_ident: (FxHashMap::default(), 0),
                    )*
                }
            }

            pub fn update_from_frame_data(&mut self, frame_data: &FrameData, window: &mut Window) {
                $(
                    if let Some((view, visible)) = &mut self.$s_view_ident {
                        if *visible {
                            if let Some(frame_data) = &frame_data.$s_view_ident {
                                view.update_from_frame_data(frame_data, window);
                            }
                        }
                    }
                )*
                $(
                    for (key, (view, visible)) in &mut self.$i_view_ident.0 {
                        if !*visible {
                            continue;
                        }
                        if let Some(frame_data) = frame_data.$i_view_ident.get(key) {
                            view.update_from_frame_data(frame_data, window);
                        }
                    }
                )*
            }

            pub fn reload_emu_state(&mut self) {
                $(
                    if let Some((view, visible)) = &self.$s_view_ident {
                        let emu_state = view.emu_state();
                        self.messages.push(Message::$s_update_emu_state_message_ident(
                            Some((emu_state, *visible)),
                        ));
                    }
                )*
                $(
                    for (key, (view, visible)) in &self.$i_view_ident.0 {
                        let emu_state = view.emu_state();
                        self.messages.push(Message::$i_update_emu_state_message_ident(
                            *key,
                            Some((emu_state, *visible)),
                        ));
                    }
                )*
            }

            pub fn render_menu(&mut self, ui: &imgui::Ui, window: &mut Window) {
                $(
                    if MenuItem::new(<$s_view_ty>::NAME)
                        .selected(self.$s_view_ident.is_some())
                        .build(ui) {
                        if let Some(view) = self.$s_view_ident.take() {
                            self.messages.push(Message::$s_update_emu_state_message_ident(
                                None,
                            ));
                            view.0.destroy(window);
                        } else {
                            let view = <$s_view_ty>::new(window);
                            let emu_state = view.emu_state();
                            self.$s_view_ident = Some((view, true));
                            self.messages.push(Message::$s_update_emu_state_message_ident(
                                Some((emu_state, true)),
                            ));
                        }
                    }
                )*
                ui.separator();
                $(
                    if MenuItem::new(<$i_view_ty>::NAME).build(ui) {
                        let key = self.$i_view_ident.1;
                        self.$i_view_ident.1 += 1;
                        let view = <$i_view_ty>::new(window);
                        let emu_state = view.emu_state();
                        self.$i_view_ident.0.insert(key, (view, true));
                        self.messages.push(Message::$i_update_emu_state_message_ident(
                            key,
                            Some((emu_state, true)),
                        ));
                    }
                )*
            }

            pub fn render<'a>(
                &'a mut self,
                ui: &imgui::Ui,
                window: &mut Window,
                emu_running: bool,
            ) -> impl Iterator<Item = Message> + 'a {
                $(
                    if let Some((view, visible)) = &mut self.$s_view_ident {
                        let mut opened = true;
                        let was_visible = *visible;
                        let mut new_emu_state = None;
                        view.customize_window(
                            ui,
                            imgui::Window::new(<$s_view_ty>::NAME).opened(&mut opened)
                        ).build(ui, || {
                            *visible = true;
                            new_emu_state = view.render(ui, window, emu_running);
                        });
                        if let Some(new_emu_state) = new_emu_state {
                            self.messages.push(Message::$s_update_emu_state_message_ident(
                                Some((new_emu_state, true))
                            ));
                        } else if !opened {
                            self.messages.push(Message::$s_update_emu_state_message_ident(
                                None,
                            ));
                            self.$s_view_ident.take().unwrap().0.destroy(window);
                        } else if was_visible != !*visible {
                            self.messages.push(Message::$s_toggle_updates_message_ident(
                                *visible,
                            ));
                        }
                    }
                )*
                $(
                    let closed_views: Vec<_> = self.$i_view_ident.0.drain_filter(
                        |key, (view, visible)| {
                            let mut opened = true;
                            let was_visible = *visible;
                            let mut new_emu_state = None;
                            view.customize_window(
                                ui,
                                imgui::Window::new(&format!("{}##{}", <$i_view_ty>::NAME, *key))
                                    .opened(&mut opened),
                            ).build(ui, || {
                                *visible = true;
                                new_emu_state = view.render(ui, window, emu_running);
                            });
                            if let Some(new_emu_state) = new_emu_state {
                                self.messages.push(Message::$i_update_emu_state_message_ident(
                                    *key,
                                    Some((new_emu_state, true))
                                ));
                            } else if !opened {
                                self.messages.push(Message::$i_update_emu_state_message_ident(
                                    *key,
                                    None,
                                ));
                                return true;
                            } else if was_visible != !*visible {
                                self.messages.push(Message::$i_toggle_updates_message_ident(
                                    *key,
                                    *visible,
                                ));
                            }
                            false
                        }
                    ).map(|(_, (view, _))| view).collect();
                    for view in closed_views {
                        view.destroy(window);
                    }
                )*
                self.messages.drain(..)
            }
        }
    };
}

declare_structs!(
    singleton cpu_state, CpuState, ToggleCpuStateUpdates, UpdateCpuStateEmuState;
    singleton spc_state, SpcState, ToggleSpcStateUpdates, UpdateSpcStateEmuState;
    instanceable cpu_memory, CpuMemory, ToggleCpuMemoryUpdates, UpdateCpuMemoryEmuState;
    instanceable cpu_disasm, CpuDisasm, ToggleCpuDisasmUpdates, UpdateCpuDisasmEmuState;
    instanceable spc_memory, SpcMemory, ToggleSpcMemoryUpdates, UpdateSpcMemoryEmuState;
    instanceable spc_disasm, SpcDisasm, ToggleSpcDisasmUpdates, UpdateSpcDisasmEmuState;
);
