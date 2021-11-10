mod counters;
pub use counters::*;
mod latched_counters;
pub use latched_counters::*;
mod oam;
pub use oam::Oam;
mod bgs_objs_mosaic;
pub mod palette;
mod render;
pub mod vram;
mod windows_math;
pub use bgs_objs_mosaic::*;
pub use windows_math::*;

use crate::{
    cpu::{bus::AccessType, dma, Irqs},
    emu::Emu,
    schedule::{self, event_slots, Schedule, Timestamp},
    utils::{bitfield_debug, zeroed_box, Zero},
    Model,
};
use palette::Palette;
use render::ScreenPixel;
use vram::Vram;

pub const VIEW_WIDTH: usize = 256;

pub const VIEW_HEIGHT_NTSC: usize = 224;
pub const VIEW_HEIGHT_PAL: usize = 239;

pub const FB_WIDTH: usize = VIEW_WIDTH << 1;
pub const FB_HEIGHT: usize = VIEW_HEIGHT_PAL << 1;

const DOT_CYCLES: u16 = 4;
const HDRAW_CYCLES: u16 = 273 * DOT_CYCLES;
// -4 for NTSC short scanlines, +4 for PAL long scanlines, equal to
// HBLANK_0_CYCLES + HDRAW_CYCLES + HBLANK_1_CYCLES;
const SCANLINE_CYCLES: u16 = 1364;

const SCANLINES_NTSC: u16 = 262;
const SCANLINES_PAL: u16 = 312;

#[repr(C, align(64))]
#[derive(Clone)]
pub struct Framebuffer(pub [u32; FB_WIDTH * FB_HEIGHT]);

unsafe impl Zero for Framebuffer {}

bitfield_debug! {
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct Status77(pub u8) {
        pub ppu1_version: u8 @ 0..=3,
        pub range_over: bool @ 6,
        pub time_over: bool @ 7,
    }
}

bitfield_debug! {
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct Status78(pub u8) {
        pub ppu2_version: u8 @ 0..=3,
        pub pal_console: bool @ 4,
        pub external_latch_flag: bool @ 6,
        pub interlace_field: bool @ 7,
    }
}

bitfield_debug! {
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct HvStatus(pub u8) {
        pub hblank: bool @ 6,
        pub vblank: bool @ 7,
    }
}

bitfield_debug! {
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct IrqControl(pub u8) {
        pub hv_irq_mode: u8 @ 4..=5,
        pub vblank_nmi_enabled: bool @ 7,
    }
}

bitfield_debug! {
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct NmiFlag(pub u8) {
        pub cpu_version: u8 @ 0..=3,
        pub nmi_triggered: bool @ 7,
    }
}

bitfield_debug! {
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct DisplayControl0(pub u8) {
        pub master_brightness: u8 @ 0..=3,
        pub forced_blank: bool @ 7,
    }
}

bitfield_debug! {
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct DisplayControl1(pub u8) {
        pub interlacing: bool @ 0,
        pub obj_v_direction_display: bool @ 1,
        pub bg_v_direction_display: bool @ 2,
        pub h_pseudo_512_mode: bool @ 3,
        pub extbg_enabled: bool @ 6,
        pub external_sync: bool @ 7,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    StartHDraw,
    StartHBlank,
    ReloadHdmas,
    StartHdmas,
    RequestVBlankNmi,
    ReloadOamAddr,
    EndScanline,
}

pub struct Ppu {
    pub(crate) frame_finished: bool,

    pub framebuffer: Box<Framebuffer>,
    bg_line_pixels: [[ScreenPixel; FB_WIDTH]; 4],
    main_screen_line: [ScreenPixel; VIEW_WIDTH],
    sub_screen_line: [ScreenPixel; VIEW_WIDTH],
    obj_line_pixels: [ScreenPixel; VIEW_WIDTH],
    layer_window_masks: [[WindowMask; 2]; 6],
    obj_tiles_in_time: u8,

    fb_height: usize,
    view_height: usize,
    prev_line_fb_x_shift: bool,
    drawing_fb_x_shift: bool,
    fb_x_shift: bool,

    ppu1_mdr: u8,
    ppu2_mdr: u8,

    pub vram: Vram,
    pub oam: Oam,
    pub palette: Palette,

    status77: Status77,
    status78: Status78,
    hv_status: HvStatus,

    pub counters: Counters,
    latched_counters: LatchedCounters,

    vblank_nmi_enabled: bool,
    nmi_flag: NmiFlag,

    display_control_0: DisplayControl0,
    master_brightness: u8,
    display_control_1: DisplayControl1,
    pub enabled_main_screen_layers: u8,
    pub enabled_sub_screen_layers: u8,

    color_math_control_a: ColorMathControlA,
    color_math_control_b: ColorMathControlB,
    color_math_main_screen_mask: u8,
    pub sub_backdrop_color: u16,

    pub window_ranges: [(u8, u8); 2],
    win12_areas: [LayerWin12Areas; 3],
    win12_masks: [LayerWin12Masks; 2],
    pub win_disabled_layer_masks: [u8; 2],
    layer_win12_areas: [[LayerWin12Area; 2]; 6],
    layer_win12_masks: [LayerWin12Mask; 6],

    mosaic_control: MosaicControl,
    mosaic_remaining_lines: (u8, u8),
    mosaic_size: u8,
    bg_mosaic_mask: u8,

    bg_char_control_12: BgCharControl,
    bg_char_control_34: BgCharControl,
    bg_scroll_prev_1: u8,
    bg_scroll_prev_2: u8,
    pub bgs: [Bg; 4],

    bg_mode_control: BgModeControl,
    bg_mode: BgMode,
    bg_tile_size_mask: u8,

    obj_control: ObjControl,
    obj_char_base_bytes: u16,
}

impl Ppu {
    pub(crate) fn new(model: Model, schedule: &mut Schedule) -> Self {
        schedule.set_event(event_slots::PPU, schedule::Event::Ppu(Event::StartHDraw));
        schedule.schedule_event(event_slots::PPU, DOT_CYCLES as Timestamp);
        let view_height = if model == Model::Pal {
            VIEW_HEIGHT_PAL
        } else {
            VIEW_HEIGHT_NTSC
        };
        Ppu {
            frame_finished: false,

            framebuffer: zeroed_box(),
            bg_line_pixels: [[ScreenPixel(0); FB_WIDTH]; 4],
            main_screen_line: [ScreenPixel(0); VIEW_WIDTH],
            sub_screen_line: [ScreenPixel(0); VIEW_WIDTH],
            obj_line_pixels: [ScreenPixel(0); VIEW_WIDTH],
            layer_window_masks: [[[0; WIN_BUFFER_LEN]; 2]; 6],
            obj_tiles_in_time: 0,

            fb_height: view_height,
            view_height,
            prev_line_fb_x_shift: false,
            drawing_fb_x_shift: false,
            fb_x_shift: false,

            ppu1_mdr: 0,
            ppu2_mdr: 0,

            vram: Vram::new(),
            oam: Oam::new(),
            palette: Palette::new(),

            status77: Status77(0).with_ppu1_version(1),
            status78: Status78(0)
                .with_ppu2_version(2)
                .with_pal_console(model == Model::Pal),
            hv_status: HvStatus(0).with_hblank(true),

            counters: Counters::new(model, schedule),
            latched_counters: LatchedCounters::new(),

            vblank_nmi_enabled: false,
            nmi_flag: NmiFlag(0).with_cpu_version(2),

            display_control_0: DisplayControl0(0),
            master_brightness: 0,
            display_control_1: DisplayControl1(0).with_bg_v_direction_display(model == Model::Pal),
            enabled_main_screen_layers: 0,
            enabled_sub_screen_layers: 0,

            color_math_control_a: ColorMathControlA(0),
            color_math_control_b: ColorMathControlB(0),
            color_math_main_screen_mask: 0,
            sub_backdrop_color: 0,

            window_ranges: [(0, 0); 2],
            win12_areas: [LayerWin12Areas(0); 3],
            win12_masks: [LayerWin12Masks(0); 2],
            win_disabled_layer_masks: [0; 2],
            layer_win12_areas: [[LayerWin12Area::Disabled; 2]; 6],
            layer_win12_masks: [LayerWin12Mask::Or; 6],

            mosaic_control: MosaicControl(0),
            mosaic_remaining_lines: (0, 0),
            mosaic_size: 1,
            bg_mosaic_mask: 0,

            bg_char_control_12: BgCharControl(0),
            bg_char_control_34: BgCharControl(0),
            bg_scroll_prev_1: 0,
            bg_scroll_prev_2: 0,
            bgs: [Bg::new(); 4],

            bg_mode_control: BgModeControl(0),
            bg_mode: BgMode::new(0),
            bg_tile_size_mask: 0,

            obj_control: ObjControl(0),
            obj_char_base_bytes: 0,
        }
    }

    pub(crate) fn handle_event(emu: &mut Emu, event: Event, time: Timestamp) {
        match event {
            // H=1
            Event::StartHDraw => {
                if emu.ppu.counters.v_counter() == 0 {
                    emu.ppu
                        .status78
                        .set_interlace_field(!emu.ppu.status78.interlace_field());
                    emu.ppu.mosaic_remaining_lines = (emu.ppu.mosaic_size, emu.ppu.mosaic_size);
                    emu.schedule.set_event(
                        event_slots::PPU_OTHER,
                        schedule::Event::Ppu(Event::ReloadHdmas),
                    );
                    emu.schedule
                        .schedule_event(event_slots::PPU_OTHER, time + 20);
                }
                emu.ppu.hv_status.set_hblank(false);
                if emu.ppu.counters.v_counter().wrapping_sub(1) < emu.ppu.view_height as u16 {
                    if emu.ppu.counters.v_counter() == 1 {
                        emu.ppu.fb_x_shift = emu.ppu.drawing_fb_x_shift;
                        emu.ppu.prev_line_fb_x_shift = emu.ppu.drawing_fb_x_shift;
                    }
                    let line = if emu.ppu.display_control_1.interlacing() {
                        (emu.ppu.counters.v_counter() - 1) << 1
                            | emu.ppu.status78.interlace_field() as u16
                    } else {
                        emu.ppu.counters.v_counter() - 1
                    };
                    emu.ppu.render_scanline(line);
                }
                emu.schedule
                    .set_event(event_slots::PPU, schedule::Event::Ppu(Event::StartHBlank));
                emu.schedule
                    .schedule_event(event_slots::PPU, time + HDRAW_CYCLES as Timestamp);
            }

            // H=274
            Event::StartHBlank => {
                emu.ppu.hv_status.set_hblank(true);
                if emu.ppu.counters.v_counter() < emu.ppu.counters.v_display_end() {
                    emu.schedule.set_event(
                        event_slots::PPU_OTHER,
                        schedule::Event::Ppu(Event::StartHdmas),
                    );
                    emu.schedule
                        .schedule_event(event_slots::PPU_OTHER, time + 16);
                }
                emu.schedule
                    .set_event(event_slots::PPU, schedule::Event::Ppu(Event::EndScanline));
                emu.schedule.schedule_event(
                    event_slots::PPU,
                    time + (emu.ppu.counters.h_end_cycles() - (DOT_CYCLES + HDRAW_CYCLES))
                        as Timestamp,
                );
            }

            Event::ReloadHdmas => dma::Controller::reload_hdmas(emu),

            Event::StartHdmas => emu.cpu.dmac.start_hdmas(),

            // H=0, V=any
            Event::EndScanline => {
                let mut new_v_counter = emu.ppu.counters.v_counter() + 1;
                if new_v_counter == emu.ppu.counters.v_end() {
                    emu.ppu.counters.start_frame(
                        emu.ppu.view_height as u16 + 1,
                        if emu.ppu.status78.pal_console() {
                            SCANLINES_PAL
                        } else {
                            SCANLINES_NTSC
                        } + (emu.ppu.display_control_1.interlacing()
                            && !emu.ppu.status78.interlace_field())
                            as u16,
                    );
                    new_v_counter = 0;
                }
                emu.ppu.counters.start_new_line(
                    new_v_counter,
                    if emu.ppu.status78.interlace_field() {
                        if emu.ppu.status78.pal_console()
                            && emu.ppu.display_control_1.interlacing()
                            && new_v_counter == 311
                        {
                            SCANLINE_CYCLES + 4
                        } else if !emu.ppu.status78.pal_console()
                            && !emu.ppu.display_control_1.interlacing()
                            && new_v_counter == 240
                        {
                            SCANLINE_CYCLES - 4
                        } else {
                            SCANLINE_CYCLES
                        }
                    } else {
                        SCANLINE_CYCLES
                    },
                    time,
                    &mut emu.schedule,
                );
                if new_v_counter == emu.ppu.counters.v_display_end() {
                    emu.ppu.hv_status.set_vblank(true);
                    emu.ppu.frame_finished = true;
                    emu.schedule.set_event(
                        event_slots::PPU_OTHER,
                        schedule::Event::Ppu(Event::RequestVBlankNmi),
                    );
                    emu.schedule
                        .schedule_event(event_slots::PPU_OTHER, time + 2);
                    emu.schedule.schedule_event(
                        event_slots::CONTROLLERS,
                        match emu.controllers.last_auto_read() {
                            Some(last_time) => {
                                let delay = time + 130 - last_time;
                                last_time + ((delay + 255) >> 8)
                            }
                            None => time + 298,
                        },
                    );
                } else if new_v_counter == 0 {
                    emu.ppu.hv_status.set_vblank(false);
                    emu.ppu.nmi_flag.set_nmi_triggered(false);
                    if !emu.ppu.display_control_0.forced_blank() {
                        emu.ppu.status77.set_range_over(false);
                        emu.ppu.status77.set_time_over(false);
                    }
                }
                emu.schedule
                    .set_event(event_slots::PPU, schedule::Event::Ppu(Event::StartHDraw));
                emu.schedule
                    .schedule_event(event_slots::PPU, time + DOT_CYCLES as Timestamp);
            }

            // H=0.5, V=v_display_end
            Event::RequestVBlankNmi => {
                emu.ppu.nmi_flag.set_nmi_triggered(true);
                if emu.ppu.vblank_nmi_enabled {
                    emu.cpu.irqs.request_nmi(&mut emu.schedule);
                }
                emu.schedule.set_event(
                    event_slots::PPU_OTHER,
                    schedule::Event::Ppu(Event::ReloadOamAddr),
                );
                emu.schedule
                    .schedule_event(event_slots::PPU_OTHER, time + 38);
            }

            // H=10, V=v_display_end
            Event::ReloadOamAddr => {
                if !emu.ppu.display_control_0.forced_blank() {
                    emu.ppu.oam.reload_cur_byte_addr();
                }
            }
        }
    }

    #[inline]
    pub fn fb_width(&self) -> usize {
        VIEW_WIDTH << self.fb_x_shift as u8
    }

    #[inline]
    pub fn fb_height(&self) -> usize {
        self.fb_height
    }

    #[inline]
    pub fn view_height(&self) -> usize {
        self.view_height
    }

    #[inline]
    pub fn ppu1_mdr(&self) -> u8 {
        self.ppu1_mdr
    }

    #[inline]
    pub fn ppu2_mdr(&self) -> u8 {
        self.ppu2_mdr
    }

    #[inline]
    pub fn status77(&self) -> Status77 {
        self.status77
    }

    #[inline]
    pub fn status78(&mut self) -> Status78 {
        self.status78
    }

    #[inline]
    pub fn read_status77<A: AccessType>(&mut self) -> Status77 {
        let result = self.status77.0 | (self.ppu1_mdr & 0x10);
        if A::SIDE_EFFECTS {
            self.ppu1_mdr = result;
        }
        Status77(result)
    }

    #[inline]
    pub fn read_status78<A: AccessType>(&mut self) -> Status78 {
        let result = self.status78.0 | (self.ppu2_mdr & 0x20);
        if A::SIDE_EFFECTS {
            self.ppu2_mdr = result;
        }
        Status78(result)
    }

    #[inline]
    pub fn hv_status(&self) -> HvStatus {
        self.hv_status
    }

    pub fn set_irq_control(
        &mut self,
        value: IrqControl,
        irqs: &mut Irqs,
        time: Timestamp,
        schedule: &mut Schedule,
    ) {
        let prev_vblank_nmi_enabled = self.vblank_nmi_enabled;
        self.vblank_nmi_enabled = value.vblank_nmi_enabled();
        self.counters.set_hv_irq_mode(
            match value.hv_irq_mode() {
                0 => HvIrqMode::None,
                1 => HvIrqMode::HMatch,
                2 => HvIrqMode::VMatch,
                _ => HvIrqMode::VMatchHMatch,
            },
            time,
            schedule,
        );
        if !prev_vblank_nmi_enabled && self.vblank_nmi_enabled && self.nmi_flag.nmi_triggered() {
            irqs.request_nmi(schedule);
        }
    }

    #[inline]
    pub fn vblank_nmi_enabled(&self) -> bool {
        self.vblank_nmi_enabled
    }

    #[inline]
    pub fn nmi_flag(&self) -> NmiFlag {
        self.nmi_flag
    }

    #[inline]
    pub fn read_nmi_flag<A: AccessType>(&mut self) -> NmiFlag {
        let result = self.nmi_flag;
        if A::SIDE_EFFECTS {
            self.nmi_flag.set_nmi_triggered(false);
        }
        result
    }

    #[inline]
    pub fn display_control_0(&self) -> DisplayControl0 {
        self.display_control_0
    }

    pub fn set_display_control_0(&mut self, value: DisplayControl0) {
        let was_in_forced_blank = self.display_control_0.forced_blank();
        self.display_control_0 = value;
        self.master_brightness = value.master_brightness();
        if self.master_brightness != 0 {
            self.master_brightness += 1;
        };
        if was_in_forced_blank
            && !value.forced_blank()
            && self.counters.v_counter() == self.counters.v_display_end()
        {
            self.oam.reload_cur_byte_addr();
        }
    }

    fn recalc_screen_width(&mut self) {
        self.drawing_fb_x_shift =
            matches!(self.bg_mode.get(), 5 | 6) || self.display_control_1.h_pseudo_512_mode();
        self.fb_x_shift |= self.drawing_fb_x_shift;
    }

    #[inline]
    pub fn display_control_1(&self) -> DisplayControl1 {
        self.display_control_1
    }

    pub fn set_display_control_1(&mut self, value: DisplayControl1) {
        self.display_control_1 = value;
        self.recalc_screen_width();
        self.view_height = if value.bg_v_direction_display() {
            VIEW_HEIGHT_PAL
        } else {
            VIEW_HEIGHT_NTSC
        };
        self.fb_height = self.view_height << value.interlacing() as u8;
        if !value.interlacing() {
            self.status78.set_interlace_field(false);
        }
    }
}
