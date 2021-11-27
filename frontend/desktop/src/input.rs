mod editor;
pub use editor::Editor;

use core::{fmt::Write, str::FromStr};
use ness_core::controllers::joypad::Keys as EmuKeys;
use std::collections::HashMap;
use winit::event::{ElementState, Event, ScanCode, VirtualKeyCode, WindowEvent};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Changes {
    pub pressed: EmuKeys,
    pub released: EmuKeys,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriggerOp {
    And,
    Or,
    Xor,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Trigger {
    KeyCode(VirtualKeyCode),
    // TODO: Proper keyboard key to character conversion; right now winit doesn't support reading
    // the keyboard layout or the character corresponding to a key other than through virtual key
    // code mapping
    ScanCode(ScanCode, Option<VirtualKeyCode>),
    Not(Box<Trigger>),
    Chain(TriggerOp, Vec<Trigger>),
}

impl Trigger {
    fn activated(&self, pressed_keys: &[(Option<VirtualKeyCode>, ScanCode)]) -> bool {
        match self {
            Trigger::KeyCode(keycode) => pressed_keys.iter().any(|key| key.0 == Some(*keycode)),
            Trigger::ScanCode(scancode, _) => pressed_keys.iter().any(|key| key.1 == *scancode),
            Trigger::Not(trigger) => !trigger.activated(pressed_keys),
            Trigger::Chain(op, triggers) => match op {
                TriggerOp::And => triggers
                    .iter()
                    .all(|trigger| trigger.activated(pressed_keys)),
                TriggerOp::Or => triggers
                    .iter()
                    .any(|trigger| trigger.activated(pressed_keys)),
                TriggerOp::Xor => triggers
                    .iter()
                    .fold(false, |res, trigger| res ^ trigger.activated(pressed_keys)),
            },
        }
    }
}

impl ToString for Trigger {
    fn to_string(&self) -> String {
        fn write_trigger(result: &mut String, trigger: &Trigger, needs_parens_if_multiple: bool) {
            match trigger {
                &Trigger::KeyCode(key_code) => {
                    write!(result, "v{:?}", key_code).unwrap();
                }
                &Trigger::ScanCode(scan_code, key_code) => {
                    write!(result, "s{}v{:?}", scan_code, key_code).unwrap();
                }
                Trigger::Not(trigger) => {
                    result.push('!');
                    write_trigger(result, trigger, true);
                }
                Trigger::Chain(op, triggers) => {
                    if needs_parens_if_multiple {
                        result.push('(');
                    }
                    let op_str = match op {
                        TriggerOp::And => " & ",
                        TriggerOp::Or => " | ",
                        TriggerOp::Xor => " ^ ",
                    };
                    for (i, trigger) in triggers.iter().enumerate() {
                        if i != 0 {
                            result.push_str(op_str);
                        }
                        write_trigger(result, trigger, true);
                    }
                    if needs_parens_if_multiple {
                        result.push(')');
                    }
                }
            }
        }

        let mut result = String::new();
        write_trigger(&mut result, self, false);
        result
    }
}

impl FromStr for Trigger {
    type Err = ();

    fn from_str(mut s: &str) -> Result<Self, Self::Err> {
        fn parse_key_code(s: &mut &str) -> Result<VirtualKeyCode, ()> {
            use serde::{
                de::{value::Error, IntoDeserializer},
                Deserialize,
            };

            let end_index = s
                .char_indices()
                .find_map(|(i, c)| if c.is_alphanumeric() { None } else { Some(i) })
                .unwrap_or(s.len());
            let key_code_str = &s[..end_index];
            *s = &s[end_index..];

            VirtualKeyCode::deserialize(key_code_str.into_deserializer()).map_err(|_e: Error| {})
        }

        fn parse_value(s: &mut &str) -> Result<Trigger, ()> {
            let mut negate = false;
            let mut operator = None;
            let mut values = Vec::new();
            loop {
                *s = s.trim_start();

                let mut char_indices = s.char_indices();
                let next_char = char_indices.next().map(|(_, c)| c);
                if let Some((new_start_index, _)) = char_indices.next() {
                    *s = &s[new_start_index..];
                }

                let value = match next_char {
                    Some('!') => {
                        negate = true;
                        continue;
                    }

                    Some('&') => {
                        operator = Some(TriggerOp::And);
                        continue;
                    }

                    Some('|') => {
                        operator = Some(TriggerOp::Or);
                        continue;
                    }

                    Some('^') => {
                        operator = Some(TriggerOp::Xor);
                        continue;
                    }

                    Some('(') => {
                        let value = parse_value(s)?;
                        *s = s.strip_prefix(')').unwrap_or(s);
                        value
                    }

                    Some(')') => {
                        if let Some(operator) = operator {
                            if values.len() <= 1 {
                                return Err(());
                            }
                            return Ok(Trigger::Chain(operator, values));
                        } else {
                            if values.len() != 1 {
                                return Err(());
                            }
                            return Ok(values.remove(0));
                        }
                    }

                    Some('v') => Trigger::KeyCode(parse_key_code(s)?),

                    Some('s') => {
                        let mut char_indices = s.char_indices();
                        let (scan_code_end_index, scan_code_end_char) = char_indices
                            .find_map(|(i, c)| {
                                if c.is_numeric() {
                                    None
                                } else {
                                    Some((i, Some(c)))
                                }
                            })
                            .unwrap_or((s.len(), None));
                        let scan_code_str = &s[..scan_code_end_index];
                        *s = &s[scan_code_end_index..];

                        let scan_code = ScanCode::from_str(scan_code_str).map_err(drop)?;

                        let virtual_key_code = match scan_code_end_char {
                            Some('v') => Some(parse_key_code(s)?),
                            Some(c) if c.is_alphanumeric() => return Err(()),
                            _ => None,
                        };

                        Trigger::ScanCode(scan_code, virtual_key_code)
                    }

                    _ => return Err(()),
                };

                values.push(if negate {
                    Trigger::Not(Box::new(value))
                } else {
                    value
                });
                negate = false;
            }
        }

        parse_value(&mut s)
    }
}

pub struct State {
    pressed_keys: Vec<(Option<VirtualKeyCode>, ScanCode)>,
    pub keymap: HashMap<EmuKeys, Trigger>,
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
                    Trigger::Chain(
                        TriggerOp::Or,
                        vec![
                            Trigger::KeyCode(VirtualKeyCode::LShift),
                            Trigger::KeyCode(VirtualKeyCode::RShift),
                        ],
                    ),
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
        let mut new_pressed_emu_keys = EmuKeys::empty();
        for (&emu_key, trigger) in &self.keymap {
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

impl Default for State {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
