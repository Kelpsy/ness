#[cfg(feature = "imgui-log")]
mod imgui_log;
#[allow(dead_code)]
pub mod imgui_wgpu;
pub mod window;

use super::{
    config::{self, Config, LaunchConfig},
    debug_views, emu, input, triple_buffer, FrameData,
};
use ness_core::{
    ppu::{FB_HEIGHT, FB_WIDTH, VIEW_HEIGHT_NTSC, VIEW_WIDTH},
    utils::{zeroed_box, BoxedByteSlice},
};
use rfd::FileDialog;
use std::{
    env,
    fs::{self, File},
    io::{Read, Seek, SeekFrom},
    num::NonZeroU32,
    path::{Path, PathBuf},
    thread,
};

#[cfg(feature = "log")]
fn init_logging(#[cfg(feature = "imgui-log")] tx: imgui_log::Sender) -> slog::Logger {
    use slog::Drain;
    #[cfg(feature = "imgui-log")]
    {
        slog::Logger::root(imgui_log::Drain::new(tx).fuse(), slog::o!())
    }
    #[cfg(feature = "term-log")]
    {
        let decorator = slog_term::TermDecorator::new().stdout().build();
        let drain = slog_term::CompactFormat::new(decorator)
            .use_custom_timestamp(|_: &mut dyn std::io::Write| Ok(()))
            .build()
            .fuse();
        #[cfg(feature = "async-term-log")]
        {
            slog::Logger::root(
                slog_async::Async::new(drain)
                    .overflow_strategy(slog_async::OverflowStrategy::Block)
                    .thread_name("async logger".to_string())
                    .build()
                    .fuse(),
                slog::o!(),
            )
        }
        #[cfg(not(feature = "async-term-log"))]
        slog::Logger::root(std::sync::Mutex::new(drain).fuse(), slog::o!())
    }
}

struct UiState {
    global_config: Config<config::Global>,
    game_config: Option<Config<config::Game>>,

    playing: bool,
    limit_framerate: bool,

    screen_focused: bool,
    input: input::State,

    #[cfg(feature = "imgui-log")]
    console: imgui_log::Console,
    #[cfg(feature = "log")]
    logger: slog::Logger,

    frame_tx: Option<triple_buffer::Sender<FrameData>>,
    frame_rx: triple_buffer::Receiver<FrameData>,
    fps_fixed: Option<u64>,
    fb_texture_id: imgui::TextureId,
    fb_view_height: usize,
    fb_width: usize,
    fb_height: usize,

    #[cfg(feature = "debug-views")]
    debug_views: debug_views::UiState,

    message_tx: crossbeam_channel::Sender<emu::Message>,
    message_rx: crossbeam_channel::Receiver<emu::Message>,

    emu_thread: Option<thread::JoinHandle<triple_buffer::Sender<FrameData>>>,
}

impl UiState {
    fn send_message(&self, msg: emu::Message) {
        self.message_tx.send(msg).expect("Couldn't send UI message");
    }

    fn start(&mut self, config: LaunchConfig, rom: BoxedByteSlice) {
        self.stop();

        self.limit_framerate = config.limit_framerate.value;
        #[cfg(feature = "xq-audio")]
        {
            self.audio.tx_active = true;
            self.audio
                .set_xq_sample_rate_shift(config.xq_audio_sample_rate_shift);
        }

        #[cfg(feature = "log")]
        let logger = self.logger.clone();

        let frame_tx = self.frame_tx.take().unwrap();
        let message_rx = self.message_rx.clone();

        self.emu_thread = Some(
            thread::Builder::new()
                .name("emulation".to_string())
                .spawn(move || {
                    emu::main(
                        config,
                        rom,
                        frame_tx,
                        message_rx,
                        #[cfg(feature = "log")]
                        logger,
                    )
                })
                .expect("Couldn't spawn emulation thread"),
        );
    }

    fn stop(&mut self) {
        if let Some(emu_thread) = self.emu_thread.take() {
            self.send_message(emu::Message::Stop);
            self.frame_tx = Some(emu_thread.join().expect("Couldn't join emulation thread"));
        }
        self.playing = false;
    }
}

fn clear_fb_texture(id: imgui::TextureId, window: &mut window::Window) {
    let mut data = zeroed_box::<[u8; FB_WIDTH * FB_HEIGHT * 4]>();
    for i in (0..data.len()).step_by(4) {
        data[i + 3] = 0xFF;
    }
    window.gfx.imgui.texture_mut(id).set_data(
        &window.gfx.device_state.queue,
        &data[..],
        imgui_wgpu::TextureRange::default(),
    );
}

pub fn main() {
    #[cfg(feature = "imgui-log")]
    let (console, logger_tx) = imgui_log::Console::new(true);
    #[cfg(feature = "log")]
    let logger = init_logging(
        #[cfg(feature = "imgui-log")]
        logger_tx,
    );

    let config_home = match env::var_os("XDG_CONFIG_HOME") {
        Some(config_dir) => Path::new(&config_dir).join("ness"),
        None => home::home_dir()
            .map(|home| home.join(".config/ness"))
            .unwrap_or_else(|| PathBuf::from("/.config/ness")),
    };

    let global_config = if let Err(err) = fs::create_dir_all(&config_home) {
        config_error!(
            concat!(
                "Couldn't create the configuration directory{}: {}\n\nThe default configuration ",
                "will be used, new changes will not be saved.",
            ),
            match config_home.to_str() {
                Some(str) => format!(" at `{}`", str),
                None => String::new(),
            },
            err,
        );
        Config::default()
    } else {
        let path = config_home.join("global_config.json");
        let (config, save) =
            Config::<config::Global>::read_from_file_or_show_dialog(&path, "global_config.json");
        config.unwrap_or_else(|| {
            if save {
                Config {
                    contents: config::Global::default(),
                    dirty: true,
                    path: Some(path),
                }
            } else {
                Config::default()
            }
        })
    };

    let mut window_builder = futures_executor::block_on(window::Builder::new("Ness", (1300, 800)));

    let (frame_tx, frame_rx) = triple_buffer::init([
        FrameData::default(),
        FrameData::default(),
        FrameData::default(),
    ]);

    let (message_tx, message_rx) = crossbeam_channel::unbounded::<emu::Message>();

    let fb_texture_id = {
        let texture = window_builder.window.gfx.imgui.create_texture(
            &window_builder.window.gfx.device_state.device,
            &wgpu::SamplerDescriptor {
                label: Some("framebuffer sampler"),
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            },
            imgui_wgpu::TextureDescriptor {
                label: Some("framebuffer texture".to_string()),
                size: wgpu::Extent3d {
                    width: FB_WIDTH as u32,
                    height: FB_HEIGHT as u32,
                    depth_or_array_layers: 1,
                },
                format: Some(
                    if window_builder
                        .window
                        .gfx
                        .device_state
                        .surf_config
                        .format
                        .describe()
                        .srgb
                    {
                        wgpu::TextureFormat::Bgra8UnormSrgb
                    } else {
                        wgpu::TextureFormat::Bgra8Unorm
                    },
                ),
                ..Default::default()
            },
        );
        window_builder.window.gfx.imgui.add_texture(texture)
    };
    clear_fb_texture(fb_texture_id, &mut window_builder.window);

    window_builder.run(
        UiState {
            game_config: None,

            playing: false,
            limit_framerate: global_config.contents.limit_framerate,

            screen_focused: true,
            input: input::State::new(),

            #[cfg(feature = "imgui-log")]
            console,
            #[cfg(feature = "log")]
            logger,

            frame_tx: Some(frame_tx),
            frame_rx,
            fps_fixed: None,
            fb_texture_id,
            fb_view_height: VIEW_HEIGHT_NTSC,
            fb_width: VIEW_WIDTH,
            fb_height: VIEW_HEIGHT_NTSC,

            #[cfg(feature = "debug-views")]
            debug_views: debug_views::UiState::new(),

            message_tx,
            message_rx,

            emu_thread: None,
            global_config,
        },
        |_, state, event| {
            state.input.process_event(event, state.screen_focused);
        },
        |window, ui, state| {
            if state.emu_thread.is_some() {
                if let Ok(frame) = state.frame_rx.get() {
                    #[cfg(feature = "debug-views")]
                    state
                        .debug_views
                        .update_from_frame_data(&frame.debug, window);

                    let fps_fixed = (frame.fps * 10.0).round() as u64;
                    if Some(fps_fixed) != state.fps_fixed {
                        state.fps_fixed = Some(fps_fixed);
                        window
                            .window
                            .set_title(&format!("Ness - {:.01} FPS", frame.fps));
                    }

                    state.fb_view_height = frame.view_height;
                    state.fb_width = frame.fb_width;
                    state.fb_height = frame.fb_height;

                    let fb_texture = window.gfx.imgui.texture_mut(state.fb_texture_id);
                    let data = unsafe {
                        core::slice::from_raw_parts(
                            frame.fb.0.as_ptr() as *const u8,
                            FB_WIDTH * FB_HEIGHT * 4,
                        )
                    };
                    fb_texture.set_data(
                        &window.gfx.device_state.queue,
                        data,
                        imgui_wgpu::TextureRange {
                            x: 0,
                            y: 0,
                            width: NonZeroU32::new(frame.fb_width as u32),
                            height: NonZeroU32::new(frame.fb_height as u32),
                            ..imgui_wgpu::TextureRange::default()
                        },
                    );
                }
            }

            if state.playing {
                if let Some(changes) = state.input.drain_changes() {
                    state.send_message(emu::Message::UpdateInput(changes));
                }
            }

            ui.main_menu_bar(|| {
                ui.menu("Emulation", || {
                    use core::fmt::Write;

                    if imgui::MenuItem::new(if state.playing { "Pause" } else { "Play" })
                        .enabled(state.emu_thread.is_some())
                        .build(ui)
                    {
                        state.playing = !state.playing;
                        state.send_message(emu::Message::UpdatePlayingState(state.playing));
                    }

                    if imgui::MenuItem::new("Stop")
                        .enabled(state.emu_thread.is_some())
                        .build(ui)
                    {
                        state.stop();
                        clear_fb_texture(state.fb_texture_id, window);
                    }

                    if imgui::MenuItem::new("Load game...").build(ui) {
                        if let Some(path) = FileDialog::new()
                            .add_filter("SNES ROM file", &["sfc", "smc", "bin"])
                            .pick_file()
                        {
                            match config::launch_config(
                                &state.global_config.contents,
                                &config::Game::default(),
                                "TODO",
                            ) {
                                Ok(launch_config) => {
                                    let rom = {
                                        let mut rom_file = File::open(&path)
                                            .expect("Couldn't load the specified ROM file");
                                        let mut rom_len = rom_file
                                            .metadata()
                                            .expect("Couldn't get ROM file metadata")
                                            .len()
                                            as usize;
                                        if rom_len & 0x200 != 0 {
                                            rom_len -= 0x200;
                                            rom_file
                                                .seek(SeekFrom::Start(0x200))
                                                .expect("Couldn't seek in ROM file");
                                        }
                                        let mut rom = BoxedByteSlice::new_zeroed(rom_len);
                                        rom_file
                                            .read_exact(&mut rom[..])
                                            .expect("Couldn't read ROM file");
                                        rom
                                    };
                                    state.start(launch_config, rom);
                                }
                                Err(errors) => {
                                    config_error!(
                                        "Couldn't determine final configuration for game: {}",
                                        errors.into_iter().fold(String::new(), |mut acc, err| {
                                            let _ = write!(acc, "\n- {}", err);
                                            acc
                                        })
                                    );
                                }
                            }
                        }
                    }

                    if imgui::MenuItem::new("Limit framerate")
                        .build_with_ref(ui, &mut state.limit_framerate)
                    {
                        if let Some(game_config) = &mut state.game_config {
                            if let Some(limit_framerate) = &mut game_config.contents.limit_framerate
                            {
                                *limit_framerate = state.limit_framerate;
                                game_config.dirty = true;
                            }
                        }
                        state.global_config.contents.limit_framerate = state.limit_framerate;
                        state.global_config.dirty = true;
                        state.send_message(emu::Message::UpdateLimitFramerate(
                            state.limit_framerate,
                        ));
                    }
                });
                #[cfg(feature = "debug-views")]
                state.debug_views.render_menu_bar(ui, window);
            });

            #[cfg(feature = "imgui-log")]
            {
                let _window_padding = ui.push_style_var(imgui::StyleVar::WindowPadding([6.0; 2]));
                let _item_spacing = ui.push_style_var(imgui::StyleVar::ItemSpacing([0.0; 2]));
                state.console.render_window(ui, Some(window.mono_font));
            }

            #[cfg(feature = "debug-views")]
            for message in state
                .debug_views
                .render(ui, window, state.emu_thread.is_some())
            {
                state
                    .message_tx
                    .send(emu::Message::DebugViews(message))
                    .expect("Couldn't send UI message");
            }

            let style = ui.clone_style();
            let window_padding = ui.push_style_var(imgui::StyleVar::WindowPadding([0.0; 2]));
            let window_size = window.window.inner_size();
            let titlebar_height = style.frame_padding[1] * 2.0 + ui.current_font_size();
            const DEFAULT_SCALE: f32 = 2.0;
            imgui::Window::new("Screen")
                .size(
                    [
                        VIEW_WIDTH as f32 * DEFAULT_SCALE,
                        (VIEW_HEIGHT_NTSC * 2) as f32 * DEFAULT_SCALE + titlebar_height,
                    ],
                    imgui::Condition::FirstUseEver,
                )
                .position(
                    [
                        (window_size.width as f64 * 0.5 / window.scale_factor) as f32,
                        (window_size.height as f64 * 0.5 / window.scale_factor) as f32,
                    ],
                    imgui::Condition::FirstUseEver,
                )
                .position_pivot([0.5; 2])
                .build(ui, || {
                    let content_size = ui.content_region_avail();
                    let aspect_ratio = VIEW_WIDTH as f32 / state.fb_view_height as f32;
                    let width = (content_size[1] * aspect_ratio).min(content_size[0]);
                    let height = width / aspect_ratio;
                    ui.set_cursor_pos([
                        (content_size[0] - width) * 0.5,
                        titlebar_height + (content_size[1] - height) * 0.5,
                    ]);
                    imgui::Image::new(state.fb_texture_id, [width, height])
                        .uv1([
                            state.fb_width as f32 / FB_WIDTH as f32,
                            state.fb_height as f32 / FB_HEIGHT as f32,
                        ])
                        .build(ui);
                    state.screen_focused = ui.is_window_focused();
                });
            drop(window_padding);

            window::ControlFlow::Continue
        },
        move |_, mut state| {
            state.stop();
            state
                .global_config
                .flush()
                .expect("Couldn't save global configuration");
        },
    );
}
