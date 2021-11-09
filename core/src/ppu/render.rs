use super::{BgIndex, Ppu, FB_WIDTH, VIEW_WIDTH};
use crate::utils::bitfield_debug;

bitfield_debug! {
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct ScreenPixel(pub u32) {
        pub rgb: u16 @ 0..=14,
        pub bg_priority: u8 @ 16..=17,
    }
}

fn r_g_b_from_rgb5(value: u32) -> (u32, u32, u32) {
    (value & 0x1F, value >> 5 & 0x1F, value >> 10 & 0x1F)
}

impl Ppu {
    pub(super) fn render_scanline(&mut self, line: u16) {
        if self.display_control_0.forced_blank() {
            let fb_line_start = line as usize * FB_WIDTH;
            let fb_line_len = VIEW_WIDTH << self.fb_x_shift as u8;
            self.framebuffer.0[fb_line_start..fb_line_start + fb_line_len].fill(0xFF00_0000);
            return;
        }

        self.main_screen_line
            .fill(ScreenPixel(0).with_rgb(self.palette.contents[0]));
        self.sub_screen_line.fill(ScreenPixel(0));

        [
            Self::render_bgs_for_mode::<0>,
            Self::render_bgs_for_mode::<1>,
            Self::render_bgs_for_mode::<2>,
            Self::render_bgs_for_mode::<3>,
            Self::render_bgs_for_mode::<4>,
            Self::render_bgs_for_mode::<5>,
            Self::render_bgs_for_mode::<6>,
            Self::render_bgs_for_mode::<7>,
        ][self.bg_mode.get() as usize](self);

        let fb_line_start = line as usize * FB_WIDTH;
        let fb_line_drawing_len = VIEW_WIDTH << self.drawing_fb_x_shift as u8;
        let fb_line = &mut self.framebuffer.0[fb_line_start..fb_line_start + fb_line_drawing_len];
        if self.drawing_fb_x_shift {
            for (i, fb_pixels) in fb_line.array_chunks_mut::<2>().enumerate() {
                fb_pixels[0] = self.sub_screen_line[i].rgb() as u32;
                fb_pixels[1] = self.main_screen_line[i].rgb() as u32;
            }
        } else {
            for (i, fb_pixel) in fb_line.iter_mut().enumerate() {
                *fb_pixel = self.main_screen_line[i].rgb() as u32;
            }
        }

        let brightness = self.master_brightness as u32;
        for fb_pixel in fb_line {
            let (r, g, b) = r_g_b_from_rgb5(*fb_pixel);
            let r = (r * brightness) >> 4;
            let g = (g * brightness) >> 4;
            let b = (b * brightness) >> 4;
            let shifted = r << 3 | g << 11 | b << 19;
            *fb_pixel = 0xFF00_0000 | shifted | (shifted >> 5 & 0x070707)
        }

        if self.fb_x_shift && !self.prev_line_fb_x_shift {
            for fb_line_start in (0..(line as usize) * FB_WIDTH).step_by(FB_WIDTH) {
                let fb_line = &mut self.framebuffer.0[fb_line_start..fb_line_start + FB_WIDTH];
                for i in (0..FB_WIDTH).rev() {
                    fb_line[i] = fb_line[i >> 1];
                }
            }
        }

        self.prev_line_fb_x_shift = self.fb_x_shift;

        if self.fb_x_shift && !self.drawing_fb_x_shift {
            let fb_line = &mut self.framebuffer.0[fb_line_start..fb_line_start + FB_WIDTH];
            for i in (0..FB_WIDTH).rev() {
                fb_line[i] = fb_line[i >> 1];
            }
        }
    }

    fn render_bgs_for_mode<const BG_MODE: u8>(&mut self) {
        let layers_enabled = self.enabled_main_screen_layers | self.enabled_sub_screen_layers;

        let bg_2bpp_pointers = [Self::draw_bg_text::<2, 0, 0>, Self::draw_bg_text::<2, 1, 1>];
        let bg_4bpp_pointers = [Self::draw_bg_text::<4, 0, 0>, Self::draw_bg_text::<4, 1, 1>];

        if layers_enabled & 1 << 3 != 0 && BG_MODE == 0 {
            bg_2bpp_pointers[(self.bg_tile_size_mask >> 3 & 1) as usize](self, BgIndex::new(3));
        }
        if layers_enabled & 1 << 2 != 0 {
            match BG_MODE {
                0 | 1 => bg_2bpp_pointers[(self.bg_tile_size_mask >> 2 & 1) as usize](
                    self,
                    BgIndex::new(2),
                ),
                2 | 4 | 6 => {
                    // todo!("draw offset-per-tile");
                }
                _ => {}
            }
        }
        if layers_enabled & 1 << 1 != 0 {
            match BG_MODE {
                0 | 4 => bg_2bpp_pointers[(self.bg_tile_size_mask >> 1 & 1) as usize](
                    self,
                    BgIndex::new(1),
                ),
                5 => {
                    [Self::draw_bg_text::<2, 1, 0>, Self::draw_bg_text::<2, 1, 1>]
                        [(self.bg_tile_size_mask >> 1 & 1) as usize](
                        self, BgIndex::new(1)
                    );
                }
                1 | 2 | 3 => bg_4bpp_pointers[(self.bg_tile_size_mask >> 1 & 1) as usize](
                    self,
                    BgIndex::new(1),
                ),
                7 => {} // TODO: Mode 7
                _ => {}
            }
        }
        if layers_enabled & 1 << 0 != 0 {
            match BG_MODE {
                0 => bg_2bpp_pointers[(self.bg_tile_size_mask & 1) as usize](self, BgIndex::new(0)),
                1..=2 => {
                    bg_4bpp_pointers[(self.bg_tile_size_mask & 1) as usize](self, BgIndex::new(0))
                }
                3..=4 => [Self::draw_bg_text::<8, 0, 0>, Self::draw_bg_text::<8, 1, 1>]
                    [(self.bg_tile_size_mask & 1) as usize](
                    self, BgIndex::new(0)
                ),
                5 => [Self::draw_bg_text::<4, 1, 0>, Self::draw_bg_text::<4, 1, 1>]
                    [(self.bg_tile_size_mask & 1) as usize](
                    self, BgIndex::new(0)
                ),
                6 => self.draw_bg_text::<4, 1, 0>(BgIndex::new(0)),
                _ => {} // TODO: Mode 7
            }
        }

        macro_rules! render_layers {
            (
                $main_screen_layers: ident,
                $sub_screen_layers: ident,
                |$line: ident, $layers: ident, $line_pixels_bit0: ident|
                $render: expr$(,)?
            ) => {
                #[allow(clippy::unnecessary_operation)]
                {
                    let $line = &mut self.main_screen_line;
                    let $layers = self.enabled_main_screen_layers;
                    let $line_pixels_bit0 = self.fb_x_shift as usize;
                    $render;
                    if self.fb_x_shift {
                        let $line = &mut self.sub_screen_line;
                        let $layers = self.enabled_sub_screen_layers;
                        let $line_pixels_bit0 = 0;
                        $render;
                    }
                }
            };
        }

        macro_rules! copy_bg_pixels {
            (
                $i: expr,
                $dst: expr,
                $prio: literal $(mask $prio_mask: expr)?,
                $line_pixels_bit0: expr$(,)?
            ) => {
                #[allow(clippy::unused_parens)]
                for (i, dst_pixel) in $dst.iter_mut().enumerate() {
                    let color =
                        self.bg_line_pixels[$i][i << self.fb_x_shift as u8 | $line_pixels_bit0];
                    if color.bg_priority() $(& ($prio_mask | 2))* == $prio | 2 {
                        *dst_pixel = color;
                    }
                }
            };
        }

        render_layers!(
            main_screen_layers,
            sub_screen_layers,
            |line, layers, line_pixels_bit0| {
                match BG_MODE {
                    0 => {
                        if layers & 1 << 3 != 0 {
                            copy_bg_pixels!(3, line, 0, line_pixels_bit0);
                        }
                        if layers & 1 << 2 != 0 {
                            copy_bg_pixels!(2, line, 0, line_pixels_bit0);
                        }
                        if layers & 1 << 3 != 0 {
                            copy_bg_pixels!(3, line, 1, line_pixels_bit0);
                        }
                        if layers & 1 << 2 != 0 {
                            copy_bg_pixels!(2, line, 1, line_pixels_bit0);
                        }
                        if layers & 1 << 1 != 0 {
                            copy_bg_pixels!(1, line, 0, line_pixels_bit0);
                        }
                        if layers & 1 << 0 != 0 {
                            copy_bg_pixels!(0, line, 0, line_pixels_bit0);
                        }
                        if layers & 1 << 1 != 0 {
                            copy_bg_pixels!(1, line, 1, line_pixels_bit0);
                        }
                        if layers & 1 << 0 != 0 {
                            copy_bg_pixels!(0, line, 1, line_pixels_bit0);
                        }
                    }
                    1 => {
                        if layers & 1 << 2 != 0 {
                            copy_bg_pixels!(2, line, 0, line_pixels_bit0);
                        }
                        if layers & 1 << 2 != 0 && !self.bg_mode_control.bg3_m1_priority() {
                            copy_bg_pixels!(2, line, 1, line_pixels_bit0);
                        }
                        if layers & 1 << 1 != 0 {
                            copy_bg_pixels!(1, line, 0, line_pixels_bit0);
                        }
                        if layers & 1 << 0 != 0 {
                            copy_bg_pixels!(0, line, 0, line_pixels_bit0);
                        }
                        if layers & 1 << 1 != 0 {
                            copy_bg_pixels!(1, line, 1, line_pixels_bit0);
                        }
                        if layers & 1 << 0 != 0 {
                            copy_bg_pixels!(0, line, 1, line_pixels_bit0);
                        }
                        if layers & 1 << 2 != 0 && self.bg_mode_control.bg3_m1_priority() {
                            copy_bg_pixels!(2, line, 1, line_pixels_bit0);
                        }
                    }
                    2..=6 => {
                        if layers & 1 << 1 != 0 && BG_MODE != 6 {
                            copy_bg_pixels!(1, line, 0, line_pixels_bit0);
                        }
                        if layers & 1 << 0 != 0 {
                            copy_bg_pixels!(0, line, 0, line_pixels_bit0);
                        }
                        if layers & 1 << 1 != 0 && BG_MODE != 6 {
                            copy_bg_pixels!(1, line, 1, line_pixels_bit0);
                        }
                        if layers & 1 << 0 != 0 {
                            copy_bg_pixels!(0, line, 1, line_pixels_bit0);
                        }
                    }
                    _ => {
                        let extbg_enabled =
                            layers & 1 << 1 != 0 && self.display_control_1.extbg_enabled();
                        if extbg_enabled {
                            copy_bg_pixels!(1, line, 0, line_pixels_bit0);
                        }
                        if layers & 1 << 0 != 0 {
                            copy_bg_pixels!(0, line, 0 mask 0, line_pixels_bit0);
                        }
                        if extbg_enabled {
                            copy_bg_pixels!(1, line, 1, line_pixels_bit0);
                        }
                    }
                }
            }
        );
    }

    fn draw_bg_text<const COLOR_SIZE: u16, const X_SHIFT: u8, const Y_SHIFT: u8>(
        &mut self,
        bg_index: BgIndex,
    ) {
        let bg = &self.bgs[bg_index.get() as usize];

        let y = self.counters.v_counter().wrapping_add(bg.y_scroll);
        let start_x = bg.x_scroll << self.fb_x_shift as u8;

        let fb_width = VIEW_WIDTH << self.fb_x_shift as u8;
        let tile_size_x_shift = 3 + X_SHIFT;
        let tile_size_y_shift = 3 + Y_SHIFT;

        let common_pal_base = if COLOR_SIZE == 2 && self.bg_mode.get() == 0 {
            bg_index.get() << 5
        } else {
            0
        };

        let (wide_x_mask, line_screen_base_words) = if bg.screen_size & 2 != 0 {
            if bg.screen_size & 1 != 0 {
                (
                    0x20,
                    bg.screen_base_words
                        .wrapping_add((y >> tile_size_y_shift & 0x1F) << 5)
                        .wrapping_add((y >> tile_size_y_shift & 0x20) << 6),
                )
            } else {
                (
                    0,
                    bg.screen_base_words
                        .wrapping_add((y >> tile_size_y_shift & 0x3F) << 5),
                )
            }
        } else {
            (
                (bg.screen_size as u16 & 1) << 5,
                bg.screen_base_words
                    .wrapping_add((y >> tile_size_y_shift & 0x1F) << 5),
            )
        };

        let mut tile_data = [0; 65];
        let mut tiles_len = 0;
        {
            let fetch_x_mask = 0x1F | wide_x_mask;
            let mut fetch_x = start_x >> tile_size_x_shift & fetch_x_mask;
            let end_fetch_x = (start_x + (fb_width - 1) as u16) >> tile_size_x_shift & fetch_x_mask;
            loop {
                tile_data[tiles_len] = self.vram.contents.read_le::<u16>(
                    (line_screen_base_words
                        .wrapping_add(fetch_x & 0x1F)
                        .wrapping_add((fetch_x & wide_x_mask) << 5)
                        << 1) as usize,
                );
                tiles_len += 1;
                if tiles_len != 1 && fetch_x == end_fetch_x {
                    break;
                }
                fetch_x = (fetch_x + 1) & fetch_x_mask;
            }
        }

        let tile_y_mask = (1 << tile_size_y_shift) - 1;
        let y_off_in_tile_row = y & tile_y_mask;

        let tile_size_x_mask = (1 << tile_size_x_shift) - 1;
        let start_x_off_in_tile = start_x as usize & tile_size_x_mask;

        let mut first = true;
        let mut tile_pixels = [ScreenPixel(0); 16];
        let mut start_tile_x_half = (start_x as usize & tile_size_x_mask) >> 3 & 1;

        for (line_x, line_pixel) in self.bg_line_pixels[bg_index.get() as usize][..fb_width]
            .iter_mut()
            .enumerate()
        {
            let tiles_x = start_x_off_in_tile + line_x;
            if tiles_x & tile_size_x_mask == 0 || first {
                first = false;

                let tile = tile_data[tiles_x >> tile_size_x_shift];
                let y_off_in_tile = if tile & 1 << 15 != 0 {
                    tile_y_mask ^ y_off_in_tile_row
                } else {
                    y_off_in_tile_row
                };
                let char_base_bytes = bg
                    .char_base_bytes
                    .wrapping_add(
                        (tile.wrapping_add((y_off_in_tile & 8) << 1) & 0x3FF) * (COLOR_SIZE * 8),
                    )
                    .wrapping_add((y_off_in_tile & 7) << 1);
                let pal_base = match COLOR_SIZE {
                    2 => common_pal_base | ((tile >> 10 & 7) << 2) as u8,
                    4 => ((tile >> 10 & 7) << 4) as u8,
                    _ => 0,
                };
                let pixel_attrs = ScreenPixel(0).with_bg_priority(2 | (tile >> 13 & 1) as u8);
                let x_flip = if tile & 1 << 14 != 0 { 0 } else { 7 };

                let end_tile_x_half = if tile_size_x_shift == 4 && line_x + 8 < fb_width {
                    2
                } else {
                    1
                };
                let tile_x_half_flip = (tile >> 14 & 1) & (tile_size_x_shift == 4) as u16;
                for tile_half_i in start_tile_x_half..end_tile_x_half {
                    let char_base_bytes = char_base_bytes
                        .wrapping_add(COLOR_SIZE * 8 * (tile_half_i as u16 ^ tile_x_half_flip));
                    let mut pixels = [0; 8];
                    for i in 0..COLOR_SIZE {
                        let plane = self.vram.contents
                            [(char_base_bytes.wrapping_add((i & 1) | ((i & !1) << 3))) as usize];
                        for (x, pixel) in pixels.iter_mut().enumerate() {
                            *pixel |= (plane >> x & 1) << i;
                        }
                    }
                    for i in 0..8 {
                        let color_index = pixels[i ^ x_flip];
                        tile_pixels[tile_half_i << 3 | i] = if color_index != 0 {
                            pixel_attrs.with_rgb(
                                self.palette.contents[color_index.wrapping_add(pal_base) as usize],
                            )
                        } else {
                            ScreenPixel(0)
                        };
                    }
                }
                start_tile_x_half = 0;
            }
            *line_pixel = tile_pixels[tiles_x & tile_size_x_mask];
        }
    }
}
