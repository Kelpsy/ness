#[cfg(feature = "debug-views")]
use super::debug_views;
use super::{config::LaunchConfig, input, triple_buffer, FrameData};
use ness_core::{emu::Emu, utils::BoxedByteSlice, Model};
use std::{
    hint,
    time::{Duration, Instant},
};

pub enum Message {
    UpdateInput(input::Changes),
    #[cfg(feature = "debug-views")]
    DebugViews(debug_views::Message),
    UpdatePlayingState(bool),
    UpdateLimitFramerate(bool),
    Stop,
}

pub(super) fn main(
    config: LaunchConfig,
    rom: BoxedByteSlice,
    mut frame_tx: triple_buffer::Sender<FrameData>,
    message_rx: crossbeam_channel::Receiver<Message>,
    #[cfg(feature = "log")] logger: slog::Logger,
) -> triple_buffer::Sender<FrameData> {
    let mut emu = Emu::new(
        config.model,
        rom,
        #[cfg(feature = "log")]
        &logger,
    );

    let mut playing = false;
    let mut limit_framerate = config.limit_framerate.value;

    let mut last_frame_time = Instant::now();

    let mut frames_since_last_fps_calc = 0;
    let mut last_fps_calc_time = last_frame_time;
    let mut fps = 0.0;

    #[cfg(feature = "debug-views")]
    let mut debug_views = debug_views::EmuState::new();

    'outer: loop {
        for message in message_rx.try_iter() {
            match message {
                Message::UpdateInput(_changes) => {
                    // TODO: Emulator input
                }
                #[cfg(feature = "debug-views")]
                Message::DebugViews(message) => {
                    debug_views.handle_message(message);
                }
                Message::UpdatePlayingState(new_playing) => {
                    playing = new_playing;
                }
                Message::UpdateLimitFramerate(new_limit_framerate) => {
                    limit_framerate = new_limit_framerate;
                }
                Message::Stop => {
                    break 'outer;
                }
            }
        }

        let frame = frame_tx.start();

        if playing {
            emu.run_frame();
        }
        // TODO: Copy framebuffer to frame.fb.0

        #[cfg(feature = "debug-views")]
        debug_views.prepare_frame_data(&mut emu, &mut frame.debug);

        let frame_interval = match config.model {
            Model::Ntsc => Duration::from_nanos(1_000_000_000 / 60),
            Model::Pal => Duration::from_nanos(1_000_000_000 / 50),
        };
        const FPS_CALC_INTERVAL: Duration = Duration::from_secs(1);

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

        if limit_framerate || !playing {
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

    frame_tx
}
