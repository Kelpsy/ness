use super::Ppu;
use crate::utils::bitfield_debug;

mod bounded {
    use crate::utils::bounded_int;
    bounded_int!(pub struct BgIndex(u8), max 3);
    bounded_int!(pub(in super::super) struct BgMode(u8), max 7);
}
pub use bounded::BgIndex;
pub(super) use bounded::BgMode;

bitfield_debug! {
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct BgModeControl(pub u8) {
        pub bg_mode: u8 @ 0..=2,
        pub bg3_m1_priority: bool @ 3,
        pub bg_tile_size_mask: u8 @ 4..=7,
        pub bg1_tile_size: bool @ 4,
        pub bg2_tile_size: bool @ 5,
        pub bg3_tile_size: bool @ 6,
        pub bg4_tile_size: bool @ 7,
    }
}

bitfield_debug! {
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct BgScreenControl(pub u8) {
        pub screen_size: u8 @ 0..=1,
        pub screen_base: u8 @ 2..=7,
    }
}

bitfield_debug! {
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct BgCharControl(pub u8) {
        pub bg13_char_base: u8 @ 0..=3,
        pub bg24_char_base: u8 @ 4..=7,
    }
}

bitfield_debug! {
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct ObjControl(pub u8) {
        pub char_base_addr: u8 @ 0..=2,
        pub obj_0ff_100_gap: u8 @ 3..=4,
        pub size: u8 @ 5..=7,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bg {
    screen_control: BgScreenControl,
    pub(super) screen_base_words: u16,
    pub(super) char_base_bytes: u16,
    pub(super) screen_size: u8,
    pub x_scroll: u16,
    pub y_scroll: u16,
}

impl Bg {
    pub(super) const fn new() -> Self {
        Bg {
            screen_control: BgScreenControl(0),
            screen_base_words: 0,
            char_base_bytes: 0,
            screen_size: 0,
            x_scroll: 0,
            y_scroll: 0,
        }
    }

    #[inline]
    pub fn screen_control(&self) -> BgScreenControl {
        self.screen_control
    }

    #[inline]
    pub fn set_screen_control(&mut self, value: BgScreenControl) {
        self.screen_control = value;
        self.screen_size = value.screen_size();
        self.screen_base_words = (value.screen_base() as u16) << 10;
    }

    #[inline]
    pub fn x_scroll(&self) -> u16 {
        self.x_scroll
    }

    #[inline]
    pub fn y_scroll(&self) -> u16 {
        self.y_scroll
    }
}

impl Ppu {
    #[inline]
    pub fn bg_mode_control(&self) -> BgModeControl {
        self.bg_mode_control
    }

    pub fn set_bg_mode_control(&mut self, value: BgModeControl) {
        self.bg_mode_control = value;
        self.bg_mode = BgMode::new(value.bg_mode());
        self.bg_tile_size_mask = value.bg_tile_size_mask();
        self.recalc_screen_width();
    }

    #[inline]
    pub fn bg_char_control_12(&self) -> BgCharControl {
        self.bg_char_control_12
    }

    #[inline]
    pub fn bg_char_control_34(&self) -> BgCharControl {
        self.bg_char_control_34
    }

    #[inline]
    pub fn set_bg_char_control_12(&mut self, value: BgCharControl) {
        self.bg_char_control_12 = value;
        self.bgs[0].char_base_bytes = (value.bg13_char_base() as u16) << 13;
        self.bgs[1].char_base_bytes = (value.bg24_char_base() as u16) << 13;
    }

    #[inline]
    pub fn set_bg_char_control_34(&mut self, value: BgCharControl) {
        self.bg_char_control_34 = value;
        self.bgs[2].char_base_bytes = (value.bg13_char_base() as u16) << 13;
        self.bgs[3].char_base_bytes = (value.bg24_char_base() as u16) << 13;
    }

    #[inline]
    pub fn write_bg_x_scroll(&mut self, i: BgIndex, value: u8) {
        let bg = &mut self.bgs[i.get() as usize];
        bg.x_scroll = ((value as u16) << 8
            | (self.bg_scroll_prev_1 & !7) as u16
            | (self.bg_scroll_prev_2 & 7) as u16)
            & 0x3FF;
        self.bg_scroll_prev_1 = value;
        self.bg_scroll_prev_2 = value;
    }

    #[inline]
    pub fn write_bg_y_scroll(&mut self, i: BgIndex, value: u8) {
        let bg = &mut self.bgs[i.get() as usize];
        bg.y_scroll = ((value as u16) << 8 | self.bg_scroll_prev_1 as u16) & 0x3FF;
        self.bg_scroll_prev_1 = value;
    }

    #[inline]
    pub fn obj_control(&self) -> ObjControl {
        self.obj_control
    }

    #[inline]
    pub fn set_obj_control(&mut self, value: ObjControl) {
        self.obj_control = value;
        self.obj_char_base_bytes = (value.char_base_addr() as u16) << 14;
    }
}
