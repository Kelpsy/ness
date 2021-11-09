use super::Device;

bitflags::bitflags! {
    pub struct Keys: u16 {
        const R = 1 << 4;
        const L = 1 << 5;
        const X = 1 << 6;
        const A = 1 << 7;
        const RIGHT = 1 << 8;
        const LEFT = 1 << 9;
        const DOWN = 1 << 10;
        const UP = 1 << 11;
        const START = 1 << 12;
        const SELECT = 1 << 13;
        const Y = 1 << 14;
        const B = 1 << 15;
    }
}

pub struct Joypad {
    pub pressed_keys: Keys,
}

impl Joypad {
    pub fn new() -> Self {
        Joypad {
            pressed_keys: Keys::empty(),
        }
    }

    pub fn modify_keys(&mut self, pressed: Keys, released: Keys) {
        self.pressed_keys = (self.pressed_keys | pressed) & !released;
    }
}

impl Default for Joypad {
    fn default() -> Self {
        Self::new()
    }
}

impl Device for Joypad {
    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn auto_read(&mut self) -> u16 {
        self.pressed_keys.bits()
    }
}
