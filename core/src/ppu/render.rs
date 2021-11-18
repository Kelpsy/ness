use super::{oam, BgIndex, Ppu, FB_WIDTH, VIEW_WIDTH};
use crate::utils::bitfield_debug;

bitfield_debug! {
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct ScreenPixel(pub u32) {
        pub rgb: u16 @ 0..=14,
        pub bg_priority: u8 @ 16..=17,
        pub obj_priority: u8 @ 16..=18,
        pub color_math_mask: u8 @ 24..=29,
        pub bg1: bool @ 24,
        pub bg2: bool @ 25,
        pub bg3: bool @ 26,
        pub bg4: bool @ 27,
        pub obj_pal47: bool @ 28,
        pub backdrop: bool @ 29,
        pub forced_black: bool @ 30,
    }
}

fn r_g_b_from_rgb5(value: u32) -> (u32, u32, u32) {
    (value & 0x1F, value >> 5 & 0x1F, value >> 10 & 0x1F)
}

fn direct_color_from_index(value: u8) -> u16 {
    (value as u16 & 0xC0) << 7 | (value as u16 & 0x38) << 4 | (value as u16 & 7) << 2
}

#[inline(always)]
fn blend_pixels<
    const MAYBE_DIV2_RESULT: bool,
    const SUB_SCREEN_BG_OBJ_ENABLED: bool,
    const SUB: bool,
>(
    main: ScreenPixel,
    sub: ScreenPixel,
) -> u32 {
    let (main_r, main_g, main_b) = r_g_b_from_rgb5(main.0);
    let (sub_r, sub_g, sub_b) = r_g_b_from_rgb5(sub.0);
    let (mut r, mut g, mut b) = if SUB {
        (
            main_r.saturating_sub(sub_r),
            main_g.saturating_sub(sub_g),
            main_b.saturating_sub(sub_b),
        )
    } else {
        (
            (main_r + sub_r).min(0x1F),
            (main_g + sub_g).min(0x1F),
            (main_b + sub_b).min(0x1F),
        )
    };
    if MAYBE_DIV2_RESULT && !main.forced_black() && (!sub.backdrop() || !SUB_SCREEN_BG_OBJ_ENABLED)
    {
        r >>= 1;
        g >>= 1;
        b >>= 1;
    }
    b << 10 | g << 5 | r
}

impl Ppu {
    pub(super) fn render_scanline(&mut self, line: u16) {
        if self.display_control_0.forced_blank() {
            let fb_line_start = line as usize * FB_WIDTH;
            let fb_line_len = VIEW_WIDTH << self.fb_x_shift as u8;
            self.framebuffer.0[fb_line_start..fb_line_start + fb_line_len].fill(0xFF00_0000);
            return;
        }

        self.prepare_window_buffers();

        self.main_screen_line.0.fill(
            ScreenPixel(0)
                .with_rgb(self.palette.contents[0])
                .with_backdrop(true),
        );
        self.sub_screen_line.0.fill(
            ScreenPixel(0)
                .with_rgb(self.sub_backdrop_color)
                .with_backdrop(true),
        );

        if self.mosaic_remaining_lines.0 == 0 {
            self.mosaic_remaining_lines = (self.mosaic_size, self.mosaic_size);
        }
        self.mosaic_remaining_lines.0 -= 1;

        if (self.enabled_main_screen_layers | self.enabled_sub_screen_layers) & 1 << 4 != 0
            && self.counters.v_counter() != 0
        {
            self.obj_line_pixels.0.fill(ScreenPixel(0));

            let obj_size_shifts: [(u8, u8); 2] = [
                [(0, 0), (1, 1)],
                [(0, 0), (2, 2)],
                [(0, 0), (3, 3)],
                [(1, 1), (2, 2)],
                [(1, 1), (3, 3)],
                [(2, 2), (3, 3)],
                [(1, 2), (2, 3)],
                [(1, 2), (2, 2)],
            ][self.obj_control.size() as usize];
            let compare_y = (self.counters.v_counter() - 1) as u8;

            let mut line_objs = [(0, 0, 0, 0, 0, 0, oam::Attrs(0)); 32];
            let mut line_objs_len = 0;
            let mut i = self.oam.next_first_sprite() as usize;
            let last_sprite = i.wrapping_sub(1) & 0x7F;

            while i != last_sprite {
                let obj = &self.oam.contents[i];
                i = (i + 1) & 0x7F;
                let (width_shift, height_shift) = obj_size_shifts[obj.attrs.large_size() as usize];
                let y_in_obj = compare_y.wrapping_sub(obj.y_coord);
                if y_in_obj >= 8 << height_shift {
                    continue;
                }
                if (VIEW_WIDTH as u16 + 1..513 - (8 << width_shift)).contains(&obj.x_coord) {
                    continue;
                }
                if line_objs_len >= 32 {
                    self.status77.set_range_over(true);
                    break;
                } else {
                    line_objs[line_objs_len] = (
                        obj.x_coord,
                        width_shift,
                        if obj.attrs.y_flip() {
                            // NOTE: `width_shift` isn't a typo, non-square OBJs aren't flipped as
                            // expected
                            y_in_obj ^ ((8 << width_shift) - 1)
                        } else {
                            y_in_obj
                        },
                        obj.tile_number,
                        obj.pal_number,
                        obj.bg_prio,
                        obj.attrs,
                    );
                    line_objs_len += 1;
                }
            }

            self.obj_tiles_in_time = 0;
            for &(x, width_shift, y_in_obj, tile_number, pal_number, bg_prio, attrs) in
                line_objs[..line_objs_len].iter().rev()
            {
                self.draw_obj(
                    x,
                    width_shift,
                    y_in_obj,
                    tile_number,
                    pal_number,
                    bg_prio,
                    attrs,
                );
                if self.obj_tiles_in_time > 34 {
                    self.status77.set_time_over(true);
                    break;
                }
            }
        }

        [
            Self::render_for_bg_mode::<0>,
            Self::render_for_bg_mode::<1>,
            Self::render_for_bg_mode::<2>,
            Self::render_for_bg_mode::<3>,
            Self::render_for_bg_mode::<4>,
            Self::render_for_bg_mode::<5>,
            Self::render_for_bg_mode::<6>,
            Self::render_for_bg_mode::<7>,
        ][self.bg_mode.get() as usize](self);

        match self.color_math_control_a.force_main_screen_black() {
            1 | 2 => {
                for (i, pixel) in self.main_screen_line.0.iter_mut().enumerate() {
                    if !self.layer_window_masks[5][1].0[i] {
                        pixel.set_rgb(0);
                        pixel.set_forced_black(true);
                    }
                }
            }
            3 => {
                for pixel in &mut self.main_screen_line.0 {
                    pixel.set_rgb(0);
                    pixel.set_forced_black(true);
                }
            }
            _ => {}
        }

        let fb_line_start = line as usize * FB_WIDTH;
        let fb_line_drawing_len = VIEW_WIDTH << self.drawing_fb_x_shift as u8;
        let fb_line = &mut self.framebuffer.0[fb_line_start..fb_line_start + fb_line_drawing_len];
        if self.color_math_control_a.color_math_mode() == 3 {
            if self.drawing_fb_x_shift {
                for (i, fb_pixels) in fb_line.array_chunks_mut::<2>().enumerate() {
                    fb_pixels[0] = self.sub_screen_line.0[i].rgb() as u32;
                    fb_pixels[1] = self.main_screen_line.0[i].rgb() as u32;
                }
            } else {
                for (i, fb_pixel) in fb_line.iter_mut().enumerate() {
                    *fb_pixel = self.main_screen_line.0[i].rgb() as u32;
                }
            }
        } else {
            macro_rules! blend_pixels {
                (
                    $i: expr,
                    $main: expr, $sub: expr,
                    $maybe_div2_result: literal,
                    $sub_screen_bg_obj_enabled: literal,
                    $subtract: literal$(,)?
                ) => {{
                    let main = $main;
                    if self.layer_window_masks[5][0].0[$i]
                        && main.color_math_mask() & self.color_math_main_screen_mask != 0
                    {
                        blend_pixels::<$maybe_div2_result, $sub_screen_bg_obj_enabled, $subtract>(
                            main, $sub,
                        )
                    } else {
                        main.rgb() as u32
                    }
                }};
            }

            macro_rules! draw_blended {
                (
                    $maybe_div2_result: literal,
                    $sub_screen_bg_obj_enabled: literal,
                    $subtract: literal$(,)?
                ) => {
                    if self.drawing_fb_x_shift {
                        for (i, fb_pixels) in fb_line.array_chunks_mut::<2>().enumerate() {
                            let main = self.main_screen_line.0[i];
                            let sub = self.sub_screen_line.0[i];
                            fb_pixels[0] = blend_pixels!(
                                i,
                                sub,
                                main,
                                $maybe_div2_result,
                                $sub_screen_bg_obj_enabled,
                                $subtract,
                            );
                            fb_pixels[1] = blend_pixels!(
                                i,
                                main,
                                sub,
                                $maybe_div2_result,
                                $sub_screen_bg_obj_enabled,
                                $subtract,
                            );
                        }
                    } else {
                        for (i, fb_pixel) in fb_line.iter_mut().enumerate() {
                            *fb_pixel = blend_pixels!(
                                i,
                                self.main_screen_line.0[i],
                                self.sub_screen_line.0[i],
                                $maybe_div2_result,
                                $sub_screen_bg_obj_enabled,
                                $subtract,
                            );
                        }
                    }
                };
            }

            if self.color_math_control_b.div2_result() {
                if self.color_math_control_b.add_subtract() {
                    draw_blended!(false, false, true);
                } else {
                    draw_blended!(false, false, false);
                }
            } else {
                match (
                    self.color_math_control_a.sub_screen_bg_obj_enabled(),
                    self.color_math_control_b.add_subtract(),
                ) {
                    (false, false) => draw_blended!(false, false, false),
                    (false, true) => draw_blended!(false, false, true),
                    (true, false) => draw_blended!(false, true, false),
                    (true, true) => draw_blended!(false, true, true),
                };
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

    fn render_for_bg_mode<const BG_MODE: u8>(&mut self) {
        let layers_enabled = self.enabled_main_screen_layers | self.enabled_sub_screen_layers;

        static BG_2BPP_POINTERS: [fn(&mut Ppu, BgIndex); 2] =
            [Ppu::draw_bg_text::<2, 0, 0>, Ppu::draw_bg_text::<2, 1, 1>];
        static BG_4BPP_POINTERS: [fn(&mut Ppu, BgIndex); 2] =
            [Ppu::draw_bg_text::<4, 0, 0>, Ppu::draw_bg_text::<4, 1, 1>];

        if layers_enabled & 1 << 3 != 0 && BG_MODE == 0 {
            BG_2BPP_POINTERS[(self.bg_tile_size_mask >> 3 & 1) as usize](self, BgIndex::new(3));
        }
        if layers_enabled & 1 << 2 != 0 {
            match BG_MODE {
                0 | 1 => BG_2BPP_POINTERS[(self.bg_tile_size_mask >> 2 & 1) as usize](
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
                0 | 4 => BG_2BPP_POINTERS[(self.bg_tile_size_mask >> 1 & 1) as usize](
                    self,
                    BgIndex::new(1),
                ),
                5 => {
                    [Self::draw_bg_text::<2, 1, 0>, Self::draw_bg_text::<2, 1, 1>]
                        [(self.bg_tile_size_mask >> 1 & 1) as usize](
                        self, BgIndex::new(1)
                    );
                }
                1 | 2 | 3 => BG_4BPP_POINTERS[(self.bg_tile_size_mask >> 1 & 1) as usize](
                    self,
                    BgIndex::new(1),
                ),
                7 => self.draw_bg_mode7::<true>(),
                _ => {}
            }
        }
        if layers_enabled & 1 << 0 != 0 {
            match BG_MODE {
                0 => BG_2BPP_POINTERS[(self.bg_tile_size_mask & 1) as usize](self, BgIndex::new(0)),
                1..=2 => {
                    BG_4BPP_POINTERS[(self.bg_tile_size_mask & 1) as usize](self, BgIndex::new(0))
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
                _ => self.draw_bg_mode7::<false>(),
            }
        }

        macro_rules! render_layers {
            (
                $main_screen_layers: ident,
                $sub_screen_layers: ident,
                |$line: ident, $layers: ident, $screen_i: ident, $line_pixels_bit0: ident|
                $render: expr$(,)?
            ) => {
                #[allow(clippy::unnecessary_operation)]
                {
                    let $line = &mut self.main_screen_line.0;
                    let $layers = self.enabled_main_screen_layers;
                    let $screen_i = 0;
                    let $line_pixels_bit0 = self.fb_x_shift as usize;
                    $render;
                    if self.color_math_control_a.sub_screen_bg_obj_enabled() || self.fb_x_shift {
                        let $line = &mut self.sub_screen_line.0;
                        let $layers = self.enabled_sub_screen_layers;
                        let $screen_i = 1;
                        let $line_pixels_bit0 = 0;
                        $render;
                    }
                }
            };
        }

        macro_rules! copy_bg_pixels {
            (
                $i: expr,
                $screen_i: expr,
                $dst: expr,
                $prio: literal $(mask $prio_mask: expr)?,
                $line_pixels_bit0: expr$(,)?
            ) => {
                for (i, dst_pixel) in $dst.iter_mut().enumerate() {
                    let color =
                        self.bg_line_pixels[$i].0[i << self.fb_x_shift as u8 | $line_pixels_bit0];
                    if self.layer_window_masks[$i][$screen_i].0[i]
                        && color.bg_priority() $(& ($prio_mask | 2))* == $prio | 2
                    {
                        *dst_pixel = color;
                    }
                }
            };
        }

        macro_rules! copy_obj_pixels {
            (
                $layers: expr,
                $screen_i: expr,
                $dst: expr,
                $prio: literal $(mask $prio_mask: expr)?$(,)?
            ) => {
                if $layers & 1 << 4 != 0 && self.counters.v_counter() != 0 {
                    for (i, dst_pixel) in $dst.iter_mut().enumerate() {
                        let color = self.obj_line_pixels.0[i];
                        if self.layer_window_masks[4][$screen_i].0[i]
                            && color.obj_priority() $(& ($prio_mask | 4))* == $prio | 4
                        {
                            *dst_pixel = color;
                        }
                    }
                }
            };
        }

        render_layers!(
            main_screen_layers,
            sub_screen_layers,
            |line, layers, screen_i, line_pixels_bit0| {
                match BG_MODE {
                    0 => {
                        if layers & 1 << 3 != 0 {
                            copy_bg_pixels!(3, screen_i, line, 0, line_pixels_bit0);
                        }
                        if layers & 1 << 2 != 0 {
                            copy_bg_pixels!(2, screen_i, line, 0, line_pixels_bit0);
                        }
                        copy_obj_pixels!(layers, screen_i, line, 0);
                        if layers & 1 << 3 != 0 {
                            copy_bg_pixels!(3, screen_i, line, 1, line_pixels_bit0);
                        }
                        if layers & 1 << 2 != 0 {
                            copy_bg_pixels!(2, screen_i, line, 1, line_pixels_bit0);
                        }
                        copy_obj_pixels!(layers, screen_i, line, 1);
                        if layers & 1 << 1 != 0 {
                            copy_bg_pixels!(1, screen_i, line, 0, line_pixels_bit0);
                        }
                        if layers & 1 << 0 != 0 {
                            copy_bg_pixels!(0, screen_i, line, 0, line_pixels_bit0);
                        }
                        copy_obj_pixels!(layers, screen_i, line, 2);
                        if layers & 1 << 1 != 0 {
                            copy_bg_pixels!(1, screen_i, line, 1, line_pixels_bit0);
                        }
                        if layers & 1 << 0 != 0 {
                            copy_bg_pixels!(0, screen_i, line, 1, line_pixels_bit0);
                        }
                        copy_obj_pixels!(layers, screen_i, line, 3);
                    }
                    1 => {
                        if layers & 1 << 2 != 0 {
                            copy_bg_pixels!(2, screen_i, line, 0, line_pixels_bit0);
                        }
                        copy_obj_pixels!(layers, screen_i, line, 0);
                        if layers & 1 << 2 != 0 && !self.bg_mode_control.bg3_m1_priority() {
                            copy_bg_pixels!(2, screen_i, line, 1, line_pixels_bit0);
                        }
                        copy_obj_pixels!(layers, screen_i, line, 1);
                        if layers & 1 << 1 != 0 {
                            copy_bg_pixels!(1, screen_i, line, 0, line_pixels_bit0);
                        }
                        if layers & 1 << 0 != 0 {
                            copy_bg_pixels!(0, screen_i, line, 0, line_pixels_bit0);
                        }
                        copy_obj_pixels!(layers, screen_i, line, 2);
                        if layers & 1 << 1 != 0 {
                            copy_bg_pixels!(1, screen_i, line, 1, line_pixels_bit0);
                        }
                        if layers & 1 << 0 != 0 {
                            copy_bg_pixels!(0, screen_i, line, 1, line_pixels_bit0);
                        }
                        copy_obj_pixels!(layers, screen_i, line, 3);
                        if layers & 1 << 2 != 0 && self.bg_mode_control.bg3_m1_priority() {
                            copy_bg_pixels!(2, screen_i, line, 1, line_pixels_bit0);
                        }
                    }
                    2..=6 => {
                        if layers & 1 << 1 != 0 && BG_MODE != 6 {
                            copy_bg_pixels!(1, screen_i, line, 0, line_pixels_bit0);
                        }
                        copy_obj_pixels!(layers, screen_i, line, 0);
                        if layers & 1 << 0 != 0 {
                            copy_bg_pixels!(0, screen_i, line, 0, line_pixels_bit0);
                        }
                        copy_obj_pixels!(layers, screen_i, line, 1);
                        if layers & 1 << 1 != 0 && BG_MODE != 6 {
                            copy_bg_pixels!(1, screen_i, line, 1, line_pixels_bit0);
                        }
                        copy_obj_pixels!(layers, screen_i, line, 2);
                        if layers & 1 << 0 != 0 {
                            copy_bg_pixels!(0, screen_i, line, 1, line_pixels_bit0);
                        }
                        copy_obj_pixels!(layers, screen_i, line, 3);
                    }
                    _ => {
                        let extbg_enabled =
                            layers & 1 << 1 != 0 && self.display_control_1.extbg_enabled();
                        if extbg_enabled {
                            copy_bg_pixels!(1, screen_i, line, 0, line_pixels_bit0);
                        }
                        copy_obj_pixels!(layers, screen_i, line, 0);
                        if layers & 1 << 0 != 0 {
                            copy_bg_pixels!(0, screen_i, line, 0 mask 0, line_pixels_bit0);
                        }
                        copy_obj_pixels!(layers, screen_i, line, 1);
                        if extbg_enabled {
                            copy_bg_pixels!(1, screen_i, line, 1, line_pixels_bit0);
                        }
                        copy_obj_pixels!(layers, screen_i, line, 2 mask 2);
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

        let mosaic_size = if self.bg_mosaic_mask & 1 << bg_index.get() != 0 {
            self.mosaic_remaining_lines.1 as usize
        } else {
            1
        };

        let mut y = self.counters.v_counter();
        if self.display_control_1.interlacing() {
            y = y << self.display_control_1.interlacing() as u8
                | self.status78.interlace_field() as u16;
        }
        y = y.wrapping_add(bg.y_scroll);
        y -= y % mosaic_size as u16;
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

        let common_pixel_attrs = ScreenPixel(0).with_color_math_mask(1 << bg_index.get());
        let tile_size_x_mask = (1 << tile_size_x_shift) - 1;
        let start_x_off_in_tile = start_x as usize & tile_size_x_mask;

        let mut first = true;
        let mut mosaic_counter = 1;
        let mut pixel = ScreenPixel(0);
        let mut tile_pixels = [ScreenPixel(0); 16];
        let mut start_tile_x_half = (start_x as usize & tile_size_x_mask) >> 3 & 1;

        for (line_x, line_pixel) in self.bg_line_pixels[bg_index.get() as usize].0[..fb_width]
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
                let pixel_attrs = common_pixel_attrs.with_bg_priority(2 | (tile >> 13 & 1) as u8);
                let x_flip = if tile & 1 << 14 != 0 { 0 } else { 7 };
                let direct_color_bits = if COLOR_SIZE == 8 {
                    (tile & 0x1000) | (tile >> 5 & 0x40) | (tile >> 9 & 2)
                } else {
                    0
                };

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
                                if COLOR_SIZE == 8 && self.color_math_control_a.use_direct_color() {
                                    direct_color_from_index(color_index) | direct_color_bits
                                } else {
                                    self.palette.contents
                                        [color_index.wrapping_add(pal_base) as usize]
                                },
                            )
                        } else {
                            ScreenPixel(0)
                        };
                    }
                }
                start_tile_x_half = 0;
            }
            mosaic_counter -= 1;
            if mosaic_counter == 0 {
                mosaic_counter = mosaic_size;
                pixel = tile_pixels[tiles_x & tile_size_x_mask];
            }
            *line_pixel = pixel;
        }
    }

    fn draw_bg_mode7<const EXTBG: bool>(&mut self) {
        let mosaic_size = if self.bg_mosaic_mask & 1 << EXTBG as u8 != 0 {
            self.mosaic_remaining_lines.1 as usize
        } else {
            1
        };

        let mut screen_y = self.counters.v_counter();
        screen_y -= screen_y % mosaic_size as u16;

        let (flipped_start_x, x_incr, y_incr) = if self.mode7.control().x_flip() {
            (
                0xFF,
                -self.mode7.params[0] as i32,
                -self.mode7.params[2] as i32,
            )
        } else {
            (
                0_i16,
                self.mode7.params[0] as i32,
                self.mode7.params[2] as i32,
            )
        };
        let flipped_y = if self.mode7.control().y_flip() {
            0xFF ^ screen_y
        } else {
            screen_y
        } as i16;

        let origin = self.mode7.origin();

        let mut map_x = ((origin[0] as i32 * self.mode7.params[0] as i32) & !0x3F)
            + ((origin[1] as i32 * self.mode7.params[1] as i32) & !0x3F)
            + ((self.mode7.center[0] as i32) << 8)
            + (flipped_start_x as i32 * self.mode7.params[0] as i32)
            + ((flipped_y as i32 * self.mode7.params[1] as i32) & !0x3F);
        let mut map_y = ((origin[0] as i32 * self.mode7.params[2] as i32) & !0x3F)
            + ((origin[1] as i32 * self.mode7.params[3] as i32) & !0x3F)
            + ((self.mode7.center[1] as i32) << 8)
            + (flipped_start_x as i32 * self.mode7.params[2] as i32)
            + ((flipped_y as i32 * self.mode7.params[3] as i32) & !0x3F);

        let mut pixel_attrs = ScreenPixel(0).with_color_math_mask(1 << EXTBG as u8);
        if !EXTBG {
            pixel_attrs.set_bg_priority(2);
        }
        let mut mosaic_counter = 1;
        let mut pixel = ScreenPixel(0);

        for line_pixels in self.bg_line_pixels[EXTBG as usize].0
            [..VIEW_WIDTH << self.fb_x_shift as usize]
            .chunks_exact_mut(self.fb_x_shift as usize + 1)
        {
            mosaic_counter -= 1;
            #[allow(clippy::never_loop)]
            if mosaic_counter == 0 {
                mosaic_counter = mosaic_size;
                pixel = loop {
                    let screen_over = map_x as u32 > 0x80 << 11 || map_y as u32 > 0x80 << 11;
                    let tile_index =
                        ((map_y >> 11) as u16 & 0x7F) << 7 | ((map_x >> 11) as u16 & 0x7F);
                    let mut char_index = self.vram.contents[(tile_index << 1) as usize];
                    match self.mode7.control().screen_over() {
                        0 | 1 => {}
                        2 => {
                            if screen_over {
                                break ScreenPixel(0);
                            }
                        }
                        _ => {
                            if screen_over {
                                char_index = 0;
                            }
                        }
                    }
                    let color_index = self.vram.contents[((char_index as u16) << 7
                        | (map_y as u16 >> 4 & 0x70)
                        | (map_x as u16 >> 7 & 0xE)
                        | 1) as usize];
                    break if EXTBG {
                        if color_index & 0x7F == 0 {
                            ScreenPixel(0)
                        } else {
                            pixel_attrs
                                .with_bg_priority(2 | (color_index >> 7))
                                .with_rgb(self.palette.contents[(color_index & 0x7F) as usize])
                        }
                    } else if color_index == 0 {
                        ScreenPixel(0)
                    } else {
                        pixel_attrs.with_rgb(if self.color_math_control_a.use_direct_color() {
                            direct_color_from_index(color_index)
                        } else {
                            self.palette.contents[color_index as usize]
                        })
                    };
                };
            }
            line_pixels.fill(pixel);
            map_x += x_incr;
            map_y += y_incr;
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_obj(
        &mut self,
        obj_start_line_x: u16,
        width_shift: u8,
        y_in_obj: u8,
        base_tile_number: u8,
        pal_number: u8,
        bg_prio: u8,
        attrs: oam::Attrs,
    ) {
        let (start_x, end_x, mut line_x) = if obj_start_line_x >= VIEW_WIDTH as u16 {
            (512 - obj_start_line_x as usize, 8 << width_shift, 0)
        } else {
            (
                0,
                (obj_start_line_x as usize + (8 << width_shift)).min(VIEW_WIDTH)
                    - obj_start_line_x as usize,
                obj_start_line_x as usize,
            )
        };

        let char_base_bytes = self
            .obj_char_base_bytes
            .wrapping_add(if attrs.tile_table() {
                (1 + self.obj_control.obj_0ff_100_gap() as u16) << 13
            } else {
                0
            })
            | (y_in_obj as u16 & 7) << 1;
        let pal_base = 0x80 | pal_number << 4;
        let line_base_tile_number = base_tile_number.wrapping_add(y_in_obj >> 3 << 4);

        let pixel_attrs = ScreenPixel(0)
            .with_obj_priority(4 | bg_prio)
            .with_obj_pal47(pal_number >= 4);

        let mut first = true;
        let mut tile_pixels = [0; 8];
        let (x_in_tile_flip, tile_x_flip) = if attrs.x_flip() {
            (0, (1 << width_shift) - 1)
        } else {
            (7, 0)
        };

        for x in start_x..end_x {
            if x & 7 == 0 || first {
                first = false;
                self.obj_tiles_in_time += 1;
                if self.obj_tiles_in_time > 34 {
                    return;
                }
                let char_base_bytes = char_base_bytes.wrapping_add(
                    (line_base_tile_number.wrapping_add(((x >> 3) ^ tile_x_flip) as u8) as u16)
                        << 5,
                );
                tile_pixels.fill(0);
                for i in 0..4 {
                    let plane = self.vram.contents
                        [(char_base_bytes.wrapping_add((i & 1) | ((i & !1) << 3))) as usize];
                    for (x, pixel) in tile_pixels.iter_mut().enumerate() {
                        *pixel |= (plane >> (x ^ x_in_tile_flip) & 1) << i;
                    }
                }
            }
            let color_index = tile_pixels[x as usize & 7];
            if color_index != 0 {
                let color = self.palette.contents[(pal_base | color_index) as usize];
                self.obj_line_pixels.0[line_x] = pixel_attrs.with_rgb(color);
            }
            line_x += 1;
        }
    }
}
