#![feature(drain_filter, adt_const_params, array_chunks)]
#![allow(incomplete_features)]

use serde::{Deserialize, Serialize};

pub extern crate emu_utils as utils;

pub mod cart;
pub mod controllers;
pub mod cpu;
pub mod emu;
pub mod ppu;
pub mod schedule;
mod wram;
pub use wram::Wram;
pub mod apu;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Model {
    Ntsc,
    Pal,
}
