use super::Ppu;
use crate::{schedule::Timestamp, utils::bitfield_debug};

bitfield_debug! {
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct Mode7Control(pub u8) {
        pub x_flip: bool @ 0,
        pub y_flip: bool @ 1,
        pub screen_over: u8 @ 6..=7,
    }
}

pub struct Mode7 {
    old: u8,
    control: Mode7Control,
    pub params: [i16; 4],
    pub scroll: [i16; 2],
    pub center: [i16; 2],
}

impl Mode7 {
    pub(crate) const fn new() -> Self {
        Mode7 {
            old: 0,
            control: Mode7Control(0),
            params: [-1; 4],
            scroll: [-1; 2],
            center: [-1; 2],
        }
    }

    #[inline]
    pub fn control(&self) -> Mode7Control {
        self.control
    }

    pub(super) fn origin(&self) -> [i16; 2] {
        fn mask_to_1c00(value: i16) -> i16 {
            if value < 0 {
                value | 0x1C00
            } else {
                value & !0x1C00
            }
        }
        [
            mask_to_1c00(self.scroll[0] - self.center[0]),
            mask_to_1c00(self.scroll[1] - self.center[1]),
        ]
    }
}

impl Ppu {
    pub fn multiplication_result(&mut self, time: Timestamp) -> u32 {
        // TODO: This is really haphazard, is there a better way to implement it?
        let next_scanline_start_time =
            self.counters.v_counter_last_change_time() + self.counters.h_end_cycles() as Timestamp;
        if self.bg_mode.get() != 7
            || self.display_control_0.forced_blank()
            || if time < next_scanline_start_time - 12 {
                self.counters.v_counter() >= self.counters.v_display_end()
            } else {
                self.counters.v_counter() >= self.counters.v_display_end() - 1
                    && self.counters.v_counter() != self.counters.v_end() - 1
            }
        {
            return (self.mode7.params[0] as i32 * (self.mode7.params[1] >> 8) as i32) as u32
                & 0xFF_FFFF;
        }
        (if time < next_scanline_start_time {
            match (next_scanline_start_time - time) >> 1 {
                6 => (self.mode7.params[0] as i32 * self.mode7.origin()[0] as i32) >> 3,
                5 => (self.mode7.params[3] as i32 * self.mode7.origin()[1] as i32) >> 3,
                4 => (self.mode7.params[1] as i32 * self.mode7.origin()[1] as i32) >> 3,
                3 => (self.mode7.params[2] as i32 * self.mode7.origin()[0] as i32) >> 3,
                2 => {
                    let mut y = self.counters.v_counter() as i32;
                    y -= y % self.mosaic_size as i32;
                    if self.mode7.control.y_flip() {
                        y ^= 0xFF;
                    }
                    (y * self.mode7.params[1] as i32) >> 3
                }
                1 => {
                    let mut y = self.counters.v_counter() as i32;
                    y -= y % self.mosaic_size as i32;
                    if self.mode7.control.y_flip() {
                        y ^= 0xFF;
                    }
                    (y * self.mode7.params[3] as i32) >> 3
                }
                _ => {
                    let mut screen_x = self.counters.h_dot(time) as i32 & 0xFF;
                    if self.mode7.control.x_flip() {
                        screen_x ^= 0xFF;
                    }
                    (screen_x
                        * self.mode7.params
                            [(time - self.counters.v_counter_last_change_time()) as usize & 2]
                            as i32)
                        >> 3
                }
            }
        } else {
            let mut screen_x = (time - next_scanline_start_time) as i32 >> 2 & 0xFF;
            if self.mode7.control.x_flip() {
                screen_x ^= 0xFF;
            }
            (screen_x * self.mode7.params[(time - next_scanline_start_time) as usize & 2] as i32)
                >> 3
        }) as u32
            & 0xFF_FFFF
    }

    #[inline]
    pub fn set_mode7_control(&mut self, value: Mode7Control) {
        self.mode7.control = value;
    }

    #[inline]
    pub fn write_mode7_param(&mut self, i: usize, value: u8) {
        self.mode7.params[i] = (value as i16) << 8 | self.mode7.old as i16;
        self.mode7.old = value;
    }

    #[inline]
    pub fn write_mode7_scroll(&mut self, i: usize, value: u8) {
        self.mode7.scroll[i] = ((value as i16) << 8 | self.mode7.old as i16) << 3 >> 3;
        self.mode7.old = value;
    }

    #[inline]
    pub fn write_mode7_center(&mut self, i: usize, value: u8) {
        self.mode7.center[i] = ((value as i16) << 8 | self.mode7.old as i16) << 3 >> 3;
        self.mode7.old = value;
    }
}
