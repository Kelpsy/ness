#![feature(step_trait, once_cell, hash_drain_filter)]

#[macro_use]
mod utils;

mod config;
#[cfg(feature = "debug-views")]
mod debug_views;
mod input;
mod triple_buffer;

mod emu;
mod ui;

use ness_core::{ppu::Framebuffer, utils::zeroed_box};
use std::panic;

struct FrameData {
    fb: Box<Framebuffer>,
    view_height: usize,
    fb_width: usize,
    fb_height: usize,
    fps: f64,
    #[cfg(feature = "debug-views")]
    debug: debug_views::FrameData,
}

impl Default for FrameData {
    fn default() -> Self {
        FrameData {
            fb: zeroed_box(),
            fb_width: 0,
            fb_height: 0,
            view_height: 0,
            fps: 0.0,
            #[cfg(feature = "debug-views")]
            debug: debug_views::FrameData::new(),
        }
    }
}

fn main() {
    let panic_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        error!(
            "Unexpected panic",
            "Encountered unexpected panic: {}\n\nThe emulator will now quit.", info
        );
        panic_hook(info);
    }));

    ui::main();
}
