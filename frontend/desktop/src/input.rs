mod editor;
pub use editor::Editor;
mod keymap;
pub mod trigger;
pub use keymap::Keymap;

use super::config::Config;
use ness_core::controllers::joypad::Keys as EmuKeys;
use winit::event::{ElementState, Event, ScanCode, VirtualKeyCode, WindowEvent};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Changes {
    pub pressed: EmuKeys,
    pub released: EmuKeys,
}

type PressedKey = (Option<VirtualKeyCode>, ScanCode);

pub struct State {
    pressed_keys: Vec<PressedKey>,
    pub keymap: Config<Keymap>,
    pressed_emu_keys: EmuKeys,
}

impl State {
    pub fn new(keymap: Config<Keymap>) -> Self {
        State {
            pressed_keys: vec![],
            keymap,
            pressed_emu_keys: EmuKeys::empty(),
        }
    }

    pub fn process_event<T: 'static>(&mut self, event: &Event<T>, catch_new: bool) {
        if let Event::WindowEvent { event, .. } = event {
            match event {
                WindowEvent::KeyboardInput {
                    input,
                    is_synthetic: false,
                    ..
                } => {
                    let key = (input.virtual_keycode, input.scancode);
                    if input.state == ElementState::Released {
                        if let Some(i) = self.pressed_keys.iter().position(|k| *k == key) {
                            self.pressed_keys.remove(i);
                        }
                    } else if catch_new && !self.pressed_keys.contains(&key) {
                        self.pressed_keys.push(key);
                    }
                }
                WindowEvent::Focused(false) => self.pressed_keys.clear(),
                _ => {}
            }
        }
    }

    pub fn drain_changes(&mut self) -> Option<Changes> {
        let mut new_pressed_emu_keys = EmuKeys::empty();
        for (&emu_key, trigger) in &self.keymap.contents.0 {
            new_pressed_emu_keys.set(emu_key, trigger.activated(&self.pressed_keys));
        }

        if new_pressed_emu_keys != self.pressed_emu_keys {
            let pressed = new_pressed_emu_keys & !self.pressed_emu_keys;
            let released = self.pressed_emu_keys & !new_pressed_emu_keys;
            self.pressed_emu_keys = new_pressed_emu_keys;
            Some(Changes { pressed, released })
        } else {
            None
        }
    }
}
