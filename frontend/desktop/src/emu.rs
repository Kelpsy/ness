#[cfg(feature = "debug-views")]
use super::debug_views;
use super::{audio, config::LaunchConfig, input, triple_buffer, FrameData};
use ness_core::{apu::dsp::DummyBackend as DummyAudioBackend, cart::Cart, emu::Emu, Model};
use parking_lot::RwLock;
use std::{
    fs, hint,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

pub struct SharedState {
    pub playing: AtomicBool,
    pub limit_framerate: AtomicBool,
    pub autosave_interval: RwLock<Duration>,
}

pub enum Message {
    UpdateInput(input::Changes),
    UpdateSavePath(Option<PathBuf>),
    UpdateAudioSync(bool),
    #[cfg(feature = "debug-views")]
    DebugViews(debug_views::Message),
    Stop,
}

pub(super) fn main(
    config: LaunchConfig,
    cart: Cart,
    audio_tx_data: Option<audio::SenderData>,
    mut frame_tx: triple_buffer::Sender<FrameData>,
    message_rx: crossbeam_channel::Receiver<Message>,
    shared_state: Arc<SharedState>,
    #[cfg(feature = "log")] logger: slog::Logger,
) -> triple_buffer::Sender<FrameData> {
    let mut emu = Emu::new(
        config.model,
        cart,
        match &audio_tx_data {
            Some(data) => Box::new(audio::Sender::new(data, config.sync_to_audio.value)),
            None => Box::new(DummyAudioBackend),
        },
        4, // TODO: Make configurable?
        #[cfg(feature = "log")]
        &logger,
    );

    let frame_interval = match config.model {
        Model::Ntsc => Duration::from_nanos(1_000_000_000 / 60),
        Model::Pal => Duration::from_nanos(1_000_000_000 / 50),
    };
    let mut last_frame_time = Instant::now();

    const FPS_CALC_INTERVAL: Duration = Duration::from_secs(1);
    let mut frames_since_last_fps_calc = 0;
    let mut last_fps_calc_time = last_frame_time;
    let mut fps = 0.0;

    let mut cur_save_path = config.cur_save_path;
    let mut last_save_flush_time = last_frame_time;

    #[cfg(feature = "debug-views")]
    let mut debug_views = debug_views::EmuState::new();

    macro_rules! save {
        ($save_path: expr) => {
            if emu.cart.ram_modified()
                && $save_path
                    .parent()
                    .map(|parent| fs::create_dir_all(parent).is_ok())
                    .unwrap_or(true)
                && fs::write($save_path, &emu.cart.ram()[..]).is_ok()
            {
                emu.cart.mark_ram_flushed();
            }
        };
    }

    'outer: loop {
        for message in message_rx.try_iter() {
            match message {
                Message::UpdateInput(changes) => {
                    if let Some(joypad) = emu.controllers.devices[0]
                        .as_any()
                        .downcast_mut::<ness_core::controllers::joypad::Joypad>()
                    {
                        joypad.modify_keys(changes.pressed, changes.released);
                    }
                }
                Message::UpdateSavePath(new_path) => {
                    // TODO: Move/remove save file
                    cur_save_path = new_path;
                }
                Message::UpdateAudioSync(new_audio_sync) => {
                    if let Some(data) = &audio_tx_data {
                        emu.apu.dsp.backend = Box::new(audio::Sender::new(data, new_audio_sync));
                    }
                }
                #[cfg(feature = "debug-views")]
                Message::DebugViews(message) => {
                    debug_views.handle_message(message);
                }
                Message::Stop => {
                    break 'outer;
                }
            }
        }

        let playing = shared_state.playing.load(Ordering::Relaxed);

        let frame = frame_tx.start();

        if playing {
            emu.run_frame();
        }
        frame.fb.0.copy_from_slice(&emu.ppu.framebuffer.0);
        frame.view_height = emu.ppu.view_height();
        frame.fb_width = emu.ppu.fb_width();
        frame.fb_height = emu.ppu.fb_height();

        #[cfg(feature = "debug-views")]
        debug_views.prepare_frame_data(&mut emu, &mut frame.debug);

        frames_since_last_fps_calc += 1;
        let now = Instant::now();
        let elapsed = now - last_fps_calc_time;
        if elapsed >= FPS_CALC_INTERVAL {
            fps = frames_since_last_fps_calc as f64 / elapsed.as_secs_f64();
            last_fps_calc_time = now;
            frames_since_last_fps_calc = 0;
        }
        frame.fps = fps;

        frame_tx.finish();

        if let Some(save_path) = &cur_save_path {
            let now = Instant::now();
            if now - last_save_flush_time >= *shared_state.autosave_interval.read() {
                last_save_flush_time = now;
                save!(save_path);
            }
        }

        if !playing || shared_state.limit_framerate.load(Ordering::Relaxed) {
            let now = Instant::now();
            let elapsed = now - last_frame_time;
            if elapsed < frame_interval {
                last_frame_time += frame_interval;
                let sleep_interval =
                    (frame_interval - elapsed).saturating_sub(Duration::from_millis(1));
                if !sleep_interval.is_zero() {
                    std::thread::sleep(sleep_interval);
                }
                while Instant::now() < last_frame_time {
                    hint::spin_loop();
                }
            } else {
                last_frame_time = now;
            }
        }
    }

    if let Some(save_path) = &cur_save_path {
        save!(save_path);
    }

    frame_tx
}
