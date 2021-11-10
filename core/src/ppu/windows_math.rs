use super::{Ppu, VIEW_WIDTH};
use crate::utils::bitfield_debug;
use core::mem;

bitfield_debug! {
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct LayerWin12Areas(pub u8) {
        pub bg13_obj_window_1: u8 @ 0..=1,
        pub bg13_obj_window_2: u8 @ 2..=3,
        pub bg24_math_window_1: u8 @ 4..=5,
        pub bg24_math_window_2: u8 @ 6..=7,
    }
}

bitfield_debug! {
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct LayerWin12Masks(pub u8) {
        pub bg1_obj: u8 @ 0..=1,
        pub bg2_math: u8 @ 2..=3,
        pub bg3: u8 @ 4..=5,
        pub bg4: u8 @ 6..=7,
    }
}

bitfield_debug! {
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct ColorMathControlA(pub u8) {
        pub use_direct_color: bool @ 0,
        pub sub_screen_bg_obj_enabled: bool @ 1,
        pub color_math_mode: u8 @ 4..=5,
        pub force_main_screen_black: u8 @ 6..=7,
    }
}

bitfield_debug! {
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct ColorMathControlB(pub u8) {
        pub main_screen_mask: u8 @ 0..=5,
        pub div2_result: bool @ 6,
        pub add_subtract: bool @ 7,
    }
}

bitfield_debug! {
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct SubBackdropColorWrite(pub u8) {
        pub intensity: u8 @ 0..=4,
        pub red: bool @ 5,
        pub green: bool @ 6,
        pub blue: bool @ 7,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LayerWin12Area {
    Disabled,
    Inside,
    Outside,
}

impl LayerWin12Area {
    fn from_raw(raw: u8) -> Self {
        match raw {
            0..=1 => LayerWin12Area::Disabled,
            2 => LayerWin12Area::Inside,
            _ => LayerWin12Area::Outside,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LayerWin12Mask {
    Or,
    And,
    Xor,
    Xnor,
}

impl LayerWin12Mask {
    fn from_raw(raw: u8) -> Self {
        match raw {
            0 => LayerWin12Mask::Or,
            1 => LayerWin12Mask::And,
            2 => LayerWin12Mask::Xor,
            _ => LayerWin12Mask::Xnor,
        }
    }
}

pub(super) const WIN_BUFFER_ENTRY_SHIFT: usize =
    3 + mem::size_of::<usize>().trailing_zeros() as usize;
pub(super) const WIN_BUFFER_BIT_MASK: usize = (1 << WIN_BUFFER_ENTRY_SHIFT) - 1;
pub(super) const WIN_BUFFER_LEN: usize = VIEW_WIDTH >> WIN_BUFFER_ENTRY_SHIFT;
pub(super) type WindowMask = [usize; WIN_BUFFER_LEN];

impl Ppu {
    #[inline]
    pub fn win12_areas_bg_12(&self) -> LayerWin12Areas {
        self.win12_areas[0]
    }

    #[inline]
    pub fn set_win12_areas_bg_12(&mut self, value: LayerWin12Areas) {
        self.win12_areas[0] = value;
        self.layer_win12_areas[0] = [
            LayerWin12Area::from_raw(value.bg13_obj_window_1()),
            LayerWin12Area::from_raw(value.bg13_obj_window_2()),
        ];
        self.layer_win12_areas[1] = [
            LayerWin12Area::from_raw(value.bg24_math_window_1()),
            LayerWin12Area::from_raw(value.bg24_math_window_2()),
        ];
    }

    #[inline]
    pub fn win12_areas_bg_34(&self) -> LayerWin12Areas {
        self.win12_areas[1]
    }

    #[inline]
    pub fn set_win12_areas_bg_34(&mut self, value: LayerWin12Areas) {
        self.win12_areas[1] = value;
        self.layer_win12_areas[2] = [
            LayerWin12Area::from_raw(value.bg13_obj_window_1()),
            LayerWin12Area::from_raw(value.bg13_obj_window_2()),
        ];
        self.layer_win12_areas[3] = [
            LayerWin12Area::from_raw(value.bg24_math_window_1()),
            LayerWin12Area::from_raw(value.bg24_math_window_2()),
        ];
    }

    #[inline]
    pub fn win12_areas_obj_math(&self) -> LayerWin12Areas {
        self.win12_areas[2]
    }

    #[inline]
    pub fn set_win12_areas_obj_math(&mut self, value: LayerWin12Areas) {
        self.win12_areas[2] = value;
        self.layer_win12_areas[4] = [
            LayerWin12Area::from_raw(value.bg13_obj_window_1()),
            LayerWin12Area::from_raw(value.bg13_obj_window_2()),
        ];
        self.layer_win12_areas[5] = [
            LayerWin12Area::from_raw(value.bg24_math_window_1()),
            LayerWin12Area::from_raw(value.bg24_math_window_2()),
        ];
    }

    #[inline]
    pub fn win12_masks_bgs(&self) -> LayerWin12Masks {
        self.win12_masks[0]
    }

    #[inline]
    pub fn set_win12_masks_bgs(&mut self, value: LayerWin12Masks) {
        self.win12_masks[0] = value;
        self.layer_win12_masks[0] = LayerWin12Mask::from_raw(value.bg1_obj());
        self.layer_win12_masks[1] = LayerWin12Mask::from_raw(value.bg2_math());
        self.layer_win12_masks[2] = LayerWin12Mask::from_raw(value.bg3());
        self.layer_win12_masks[3] = LayerWin12Mask::from_raw(value.bg4());
    }

    #[inline]
    pub fn win12_masks_obj_math(&self) -> LayerWin12Masks {
        self.win12_masks[1]
    }

    #[inline]
    pub fn set_win12_masks_obj_math(&mut self, value: LayerWin12Masks) {
        self.win12_masks[1] = value;
        self.layer_win12_masks[4] = LayerWin12Mask::from_raw(value.bg1_obj());
        self.layer_win12_masks[5] = LayerWin12Mask::from_raw(value.bg2_math());
    }

    #[inline]
    pub fn color_math_control_a(&self) -> ColorMathControlA {
        self.color_math_control_a
    }

    #[inline]
    pub fn color_math_control_b(&self) -> ColorMathControlB {
        self.color_math_control_b
    }

    #[inline]
    pub fn set_color_math_control_a(&mut self, value: ColorMathControlA) {
        self.color_math_control_a = value;
    }

    #[inline]
    pub fn set_color_math_control_b(&mut self, value: ColorMathControlB) {
        self.color_math_control_b = value;
        self.color_math_main_screen_mask = value.main_screen_mask();
    }

    #[inline]
    pub fn write_sub_backdrop_color(&mut self, value: SubBackdropColorWrite) {
        let intensity = value.intensity() as u16;
        let (mut preserve, mut intensities) = (0x7FFF, 0);
        if value.red() {
            preserve &= !0x1F;
            intensities |= intensity;
        }
        if value.green() {
            preserve &= !(0x1F << 5);
            intensities |= intensity << 5;
        }
        if value.blue() {
            preserve &= !(0x1F << 10);
            intensities |= intensity << 10;
        }
        self.sub_backdrop_color = (self.sub_backdrop_color & preserve) | intensities;
    }

    pub(super) fn prepare_window_buffers(&mut self) {
        let layers_enabled =
            self.enabled_main_screen_layers | self.enabled_sub_screen_layers | 0x20;
        let layers_disabled_by_windows =
            self.win_disabled_layer_masks[0] | self.win_disabled_layer_masks[1] | 0x20;
        let [(win1_start, win1_end), (win2_start, win2_end)] = self.window_ranges;

        for layer_i in 0..6 {
            if layers_enabled & 1 << layer_i == 0 {
                continue;
            }

            if layer_i == 5 {
                let color_math_mode = self.color_math_control_a.color_math_mode();
                let main_screen_black_mode = self.color_math_control_a.force_main_screen_black();
                let disabled = match color_math_mode {
                    0 => {
                        self.layer_window_masks[5][0].fill(usize::MAX);
                        true
                    }
                    3 => true,
                    _ => false,
                };
                match main_screen_black_mode {
                    0 | 3 => {
                        if disabled {
                            continue;
                        }
                    }
                    _ => {}
                }
            }

            let win12_areas = self.layer_win12_areas[layer_i];
            let win12_mask = self.layer_win12_masks[layer_i];
            let buffers = &mut self.layer_window_masks[layer_i];

            if layers_disabled_by_windows & 1 << layer_i == 0
                || (win12_areas[0] == LayerWin12Area::Disabled
                    && win12_areas[1] == LayerWin12Area::Disabled)
            {
                buffers[0].fill(usize::MAX);
                buffers[1].fill(usize::MAX);
                continue;
            }

            let mut win2_buffer_i = 0;

            match win12_areas[0] {
                LayerWin12Area::Disabled => {}
                LayerWin12Area::Inside => {
                    buffers[0].fill(usize::MAX);
                    for i in win1_start as usize..=win1_end as usize {
                        buffers[0][i >> WIN_BUFFER_ENTRY_SHIFT] &=
                            !(1 << (i & WIN_BUFFER_BIT_MASK));
                    }
                    win2_buffer_i = 1;
                }
                LayerWin12Area::Outside => {
                    buffers[0].fill(0);
                    for i in win1_start as usize..=win1_end as usize {
                        buffers[0][i >> WIN_BUFFER_ENTRY_SHIFT] |= 1 << (i & WIN_BUFFER_BIT_MASK);
                    }
                    win2_buffer_i = 1;
                }
            }

            let win2_buffer = &mut buffers[win2_buffer_i];
            match win12_areas[1] {
                LayerWin12Area::Disabled => {}
                LayerWin12Area::Inside => {
                    win2_buffer.fill(usize::MAX);
                    for i in win2_start as usize..=win2_end as usize {
                        win2_buffer[i >> WIN_BUFFER_ENTRY_SHIFT] &=
                            !(1 << (i & WIN_BUFFER_BIT_MASK));
                    }
                }
                LayerWin12Area::Outside => {
                    win2_buffer.fill(0);
                    for i in win2_start as usize..=win2_end as usize {
                        win2_buffer[i >> WIN_BUFFER_ENTRY_SHIFT] |= 1 << (i & WIN_BUFFER_BIT_MASK);
                    }
                }
            }

            if win12_areas[0] != LayerWin12Area::Disabled
                && win12_areas[1] != LayerWin12Area::Disabled
            {
                match win12_mask {
                    LayerWin12Mask::Or => {
                        for i in 0..WIN_BUFFER_LEN {
                            buffers[0][i] &= buffers[1][i];
                        }
                    }
                    LayerWin12Mask::And => {
                        for i in 0..WIN_BUFFER_LEN {
                            buffers[0][i] |= buffers[1][i];
                        }
                    }
                    LayerWin12Mask::Xor => {
                        for i in 0..WIN_BUFFER_LEN {
                            buffers[0][i] = !(buffers[0][i] ^ buffers[1][i]);
                        }
                    }
                    LayerWin12Mask::Xnor => {
                        for i in 0..WIN_BUFFER_LEN {
                            buffers[0][i] ^= buffers[1][i];
                        }
                    }
                }
            }

            if layer_i == 5 {
                let [outside_math_win_buffer, not_black_buffer] = buffers;
                match self.color_math_control_a.force_main_screen_black() {
                    0 | 3 => {}
                    2 => {
                        for (not_black, &outside_math_win) in not_black_buffer
                            .iter_mut()
                            .zip(outside_math_win_buffer.iter())
                        {
                            *not_black = outside_math_win;
                        }
                    }
                    _ => {
                        for (not_black, &outside_math_win) in not_black_buffer
                            .iter_mut()
                            .zip(outside_math_win_buffer.iter())
                        {
                            *not_black = !outside_math_win;
                        }
                    }
                }
                if self.color_math_control_a.color_math_mode() == 1 {
                    for entry in &mut buffers[0] {
                        *entry = !*entry;
                    }
                }
            } else if self.win_disabled_layer_masks[1] & 1 << layer_i != 0 {
                let [main_buf, sub_buf] = buffers;
                sub_buf.copy_from_slice(main_buf);
                if self.win_disabled_layer_masks[0] & 1 << layer_i == 0 {
                    main_buf.fill(usize::MAX);
                }
            } else {
                buffers[1].fill(usize::MAX);
            }
        }
    }
}
