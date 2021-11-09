use ness_core::controllers::joypad::Keys as EmuKeys;
use std::collections::HashMap;
use winit::event::{ElementState, Event, ScanCode, VirtualKeyCode, WindowEvent};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Changes {
    pub pressed: EmuKeys,
    pub released: EmuKeys,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Trigger {
    KeyCode(VirtualKeyCode),
    // TODO: Proper keyboard key to character conversion; right now winit doesn't support reading
    // the keyboard layout or the character corresponding to a key other than through virtual key
    // code mapping
    ScanCode(ScanCode, Option<VirtualKeyCode>),
    Not(Box<Trigger>),
    And(Vec<Trigger>),
    Or(Vec<Trigger>),
    Xor(Vec<Trigger>),
}

pub struct State {
    pressed_keys: Vec<(Option<VirtualKeyCode>, ScanCode)>,
    keymap: HashMap<EmuKeys, Trigger>,
    pressed_emu_keys: EmuKeys,
}

impl State {
    #[must_use]
    pub fn new() -> Self {
        // TODO: Read saved keymap
        State {
            pressed_keys: vec![],
            keymap: [
                (EmuKeys::A, Trigger::KeyCode(VirtualKeyCode::X)),
                (EmuKeys::B, Trigger::KeyCode(VirtualKeyCode::Z)),
                (EmuKeys::X, Trigger::KeyCode(VirtualKeyCode::S)),
                (EmuKeys::Y, Trigger::KeyCode(VirtualKeyCode::A)),
                (EmuKeys::L, Trigger::KeyCode(VirtualKeyCode::Q)),
                (EmuKeys::R, Trigger::KeyCode(VirtualKeyCode::W)),
                (EmuKeys::START, Trigger::KeyCode(VirtualKeyCode::Return)),
                (
                    EmuKeys::SELECT,
                    Trigger::Or(vec![
                        Trigger::KeyCode(VirtualKeyCode::LShift),
                        Trigger::KeyCode(VirtualKeyCode::RShift),
                    ]),
                ),
                (EmuKeys::RIGHT, Trigger::KeyCode(VirtualKeyCode::Right)),
                (EmuKeys::LEFT, Trigger::KeyCode(VirtualKeyCode::Left)),
                (EmuKeys::UP, Trigger::KeyCode(VirtualKeyCode::Up)),
                (EmuKeys::DOWN, Trigger::KeyCode(VirtualKeyCode::Down)),
            ]
            .into_iter()
            .collect(),
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
        fn trigger_activated(
            trigger: &Trigger,
            pressed_keys: &[(Option<VirtualKeyCode>, ScanCode)],
        ) -> bool {
            match trigger {
                Trigger::KeyCode(keycode) => pressed_keys.iter().any(|key| key.0 == Some(*keycode)),
                Trigger::ScanCode(scancode, _) => pressed_keys.iter().any(|key| key.1 == *scancode),
                Trigger::Not(trigger) => !trigger_activated(&*trigger, pressed_keys),
                Trigger::And(triggers) => triggers
                    .iter()
                    .all(|trigger| trigger_activated(trigger, pressed_keys)),
                Trigger::Or(triggers) => triggers
                    .iter()
                    .any(|trigger| trigger_activated(trigger, pressed_keys)),
                Trigger::Xor(triggers) => triggers.iter().fold(false, |res, trigger| {
                    res ^ trigger_activated(trigger, pressed_keys)
                }),
            }
        }

        let mut new_pressed_emu_keys = EmuKeys::empty();
        for (&emu_key, trigger) in &self.keymap {
            new_pressed_emu_keys.set(emu_key, trigger_activated(trigger, &self.pressed_keys));
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

impl Default for State {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
