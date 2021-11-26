#[cfg(feature = "log")]
mod console_log;

use core::str;
use js_sys::{Uint32Array, Uint8Array};
use ness_core::{
    apu::dsp, cart, controllers::joypad::Keys, emu::Emu, utils::BoxedByteSlice, Model,
};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct EmuState {
    cart_info: cart::info::Info,
    cart: cart::Cart,
    emu: Emu,
}

#[wasm_bindgen]
pub struct FrameMetadata {
    pub fb_width: usize,
    pub fb_height: usize,
    pub view_height: usize,
}

#[wasm_bindgen]
impl EmuState {
    pub fn reset(&mut self) {
        self.emu = Emu::new(
            Model::Ntsc,
            self.cart.clone(),
            Box::new(dsp::DummyBackend),
            512,
            #[cfg(feature = "log")]
            &slog::Logger::root(slog::Discard, slog::o!()),
        );
    }

    pub fn load_save(&mut self, ram_arr: Uint8Array) {
        self.emu.cart.modify_ram(|ram| {
            ram_arr.copy_to(&mut ram[..]);
        });
    }

    pub fn export_save(&self) -> Uint8Array {
        Uint8Array::from(&self.emu.cart.ram()[..])
    }

    pub fn update_input(&mut self, pressed: u16, released: u16) {
        if let Some(joypad) = self.emu.controllers.devices[0]
            .as_any()
            .downcast_mut::<ness_core::controllers::joypad::Joypad>()
        {
            joypad.modify_keys(
                Keys::from_bits_truncate(pressed),
                Keys::from_bits_truncate(released),
            );
        }
    }

    #[wasm_bindgen(getter)]
    pub fn fps_limit(&self) -> f32 {
        if self.emu.ppu.status78().pal_console() {
            50.0
        } else {
            60.0
        }
    }

    pub fn run_frame(&mut self) -> Uint32Array {
        self.emu.run_frame();
        Uint32Array::from(&self.emu.ppu.framebuffer.0[..])
    }

    pub fn frame_metadata(&self) -> FrameMetadata {
        FrameMetadata {
            fb_width: self.emu.ppu.fb_width(),
            fb_height: self.emu.ppu.fb_height(),
            view_height: self.emu.ppu.view_height(),
        }
    }
}

// Wasm-bindgen creates invalid output using a constructor, for some reason
#[wasm_bindgen]
pub fn create_emu_state(rom_arr: Uint8Array, carts_db: &[u8], boards_db: &[u8]) -> EmuState {
    console_error_panic_hook::set_once();

    let db = str::from_utf8(carts_db)
        .and_then(|carts_db_str| Ok((carts_db_str, str::from_utf8(boards_db)?)))
        .ok()
        .and_then(|(carts_db_str, boards_db_str)| {
            cart::info::db::Db::load(carts_db_str, boards_db_str).ok()
        });

    let mut rom = BoxedByteSlice::new_zeroed(rom_arr.length() as usize);
    rom_arr.copy_to(&mut rom[..]);
    let cart_info = cart::info::Info::new(
        rom.as_byte_slice(),
        db.as_ref()
            .map(|db| (db, <sha2::Sha256 as sha2::Digest>::digest(&rom[..]).into())),
    )
    .0;
    let cart = cart::Cart::new(
        rom,
        BoxedByteSlice::new_zeroed(cart_info.ram_size as usize),
        &cart_info,
    )
    .expect("Couldn't build cart");

    EmuState {
        cart_info,
        cart: cart.clone(),
        emu: Emu::new(
            Model::Ntsc,
            cart,
            Box::new(dsp::DummyBackend),
            512,
            #[cfg(feature = "log")]
            &slog::Logger::root(console_log::Console::new(), slog::o!()),
        ),
    }
}
