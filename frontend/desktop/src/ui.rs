#[cfg(feature = "log")]
mod imgui_log;
#[allow(dead_code)]
pub mod imgui_wgpu;
pub mod window;

#[cfg(feature = "debug-views")]
use super::debug_views;
use super::{
    config::{self, Config, LaunchConfig, LoggingKind},
    emu, input, triple_buffer, FrameData,
};
use ness_core::{
    cart,
    ppu::{FB_HEIGHT, FB_WIDTH, VIEW_HEIGHT_NTSC, VIEW_WIDTH},
    utils::{zeroed_box, BoxedByteSlice, ByteSlice},
};
use parking_lot::RwLock;
use rfd::FileDialog;
use std::{
    env,
    fs::{self, File},
    io::{self, Read, Seek, SeekFrom},
    num::NonZeroU32,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

#[cfg(feature = "log")]
fn init_logging(
    imgui_log: &mut Option<(imgui_log::Console, imgui_log::Sender)>,
    kind: LoggingKind,
) -> slog::Logger {
    use slog::Drain;
    match kind {
        LoggingKind::Imgui => {
            let logger_tx = if let Some((_, logger_tx)) = imgui_log {
                logger_tx.clone()
            } else {
                let (log_console, logger_tx) = imgui_log::Console::new(true);
                *imgui_log = Some((log_console, logger_tx.clone()));
                logger_tx
            };
            slog::Logger::root(imgui_log::Drain::new(logger_tx).fuse(), slog::o!())
        }
        LoggingKind::Term => {
            *imgui_log = None;
            let decorator = slog_term::TermDecorator::new().stdout().build();
            let drain = slog_term::CompactFormat::new(decorator)
                .use_custom_timestamp(|_: &mut dyn std::io::Write| Ok(()))
                .build()
                .fuse();
            slog::Logger::root(
                slog_async::Async::new(drain)
                    .overflow_strategy(slog_async::OverflowStrategy::Block)
                    .thread_name("async logger".to_string())
                    .build()
                    .fuse(),
                slog::o!(),
            )
        }
    }
}

struct UiState {
    global_config: Config<config::Global>,
    game_config: Option<Config<config::Game>>,
    cart_db: Option<cart::info::db::Db>,

    playing: bool,
    limit_framerate: config::RuntimeModifiable<bool>,

    screen_focused: bool,
    input: input::State,

    #[cfg(feature = "log")]
    imgui_log: Option<(imgui_log::Console, imgui_log::Sender)>,
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
    emu_shared_state: Option<Arc<emu::SharedState>>,
}

impl UiState {
    fn send_message(&self, msg: emu::Message) {
        self.message_tx.send(msg).expect("Couldn't send UI message");
    }

    fn start(&mut self, config: LaunchConfig, rom: BoxedByteSlice, cart_info: cart::info::Info) {
        self.stop();

        let ram = if let Some(path) = config.cur_save_path.as_deref() {
            match File::open(&path) {
                Ok(mut ram_file) => {
                    let ram_len = ram_file
                        .metadata()
                        .expect("Couldn't get save RAM file metadata")
                        .len()
                        .next_power_of_two() as usize;
                    let mut ram = BoxedByteSlice::new_zeroed(ram_len);
                    ram_file
                        .read_exact(&mut ram[..])
                        .expect("Couldn't read ROM file");
                    Some(ram)
                }
                Err(err) => match err.kind() {
                    io::ErrorKind::NotFound => None,
                    err => {
                        error!("Couldn't read save RAM file", "{:?}", err);
                        None
                    }
                },
            }
        } else {
            None
        }
        .unwrap_or_else(|| BoxedByteSlice::new_zeroed(cart_info.ram_size as usize));

        let cart = if let Some(cart) = cart::Cart::new(rom, ram, &cart_info) {
            cart
        } else {
            error!(
                "Cart creation error",
                "Couldn't create cart from the specified ROM and save RAM files"
            );
            return;
        };

        self.limit_framerate = config.limit_framerate;
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

        self.playing = !config.pause_on_launch;
        let emu_shared_state = Arc::new(emu::SharedState {
            playing: AtomicBool::new(self.playing),
            limit_framerate: AtomicBool::new(self.limit_framerate.value),
            autosave_interval: RwLock::new(Duration::from_secs_f32(
                config.autosave_interval_ms.value / 1000.0,
            )),
        });
        self.emu_shared_state = Some(Arc::clone(&emu_shared_state));
        self.emu_thread = Some(
            thread::Builder::new()
                .name("emulation".to_string())
                .spawn(move || {
                    emu::main(
                        config,
                        cart,
                        frame_tx,
                        message_rx,
                        emu_shared_state,
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
        self.emu_shared_state = None;
        if let Some(mut game_config) = self.game_config.take() {
            if let Some(dir_path) = game_config.path.as_ref().and_then(|p| p.parent()) {
                let _ = fs::create_dir_all(dir_path);
            }
            let _ = game_config.flush();
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

    macro_rules! read_db {
        ($config_field: ident, $name: literal) => {
            fs::read_to_string(&global_config.contents.$config_field).map_err(|err| {
                error!(
                    concat!("Couldn't read ", $name, " database"),
                    "Error reading database{}: {}",
                    if let Some(db_path_str) = global_config.contents.$config_field.to_str() {
                        format!("at `{}`", db_path_str)
                    } else {
                        "".to_string()
                    },
                    err,
                );
            })
        };
    }

    let cart_db = read_db!(cart_db_path, "cart")
        .and_then(|carts| Ok((carts, read_db!(board_db_path, "board")?)))
        .and_then(|(carts, boards)| {
            cart::info::db::Db::load(&carts, &boards).map_err(|err| {
                error!("Couldn't load cart database", "{}", err);
            })
        })
        .ok();

    #[cfg(feature = "log")]
    let mut imgui_log = None;
    #[cfg(feature = "log")]
    let logger = init_logging(&mut imgui_log, global_config.contents.logging_kind);

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
            cart_db,

            playing: false,
            limit_framerate: config::RuntimeModifiable::global(
                global_config.contents.limit_framerate,
            ),

            screen_focused: true,
            input: input::State::new(),

            #[cfg(feature = "log")]
            imgui_log,
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
            emu_shared_state: None,

            global_config,
        },
        |_, state, event| {
            state.input.process_event(event, state.screen_focused);
        },
        move |window, ui, state| {
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
                        let shared_state = state.emu_shared_state.as_mut().unwrap();
                        state.playing = !state.playing;
                        shared_state.playing.store(state.playing, Ordering::Relaxed);
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

                            let (cart_info, cart_header, cart_info_source) = cart::info::Info::new(
                                ByteSlice::new(&rom[..]),
                                state.cart_db.as_ref().map(|db| {
                                    (db, <sha2::Sha256 as sha2::Digest>::digest(&rom[..]).into())
                                }),
                            );

                            match cart_info_source {
                                cart::info::Source::Db => {}
                                cart::info::Source::Guess => {
                                    #[cfg(feature = "log")]
                                    slog::warn!(
                                        state.logger,
                                        "Couldn't find cart in database, guessing info"
                                    );
                                }
                                cart::info::Source::Default => {
                                    #[cfg(feature = "log")]
                                    slog::error!(
                                        state.logger,
                                        "Couldn't guess cart info, defaulting to LoROM"
                                    );
                                }
                            }

                            let game_title = cart_info
                                .title
                                .as_deref()
                                .unwrap_or_else(|| {
                                    path.file_name()
                                        .unwrap()
                                        .to_str()
                                        .expect("Non-UTF-8 ROM filename provided")
                                })
                                .to_string();

                            let game_config = {
                                let mut config_path = config_home.join("games").join(&game_title);
                                config_path.set_extension("json");
                                let (config, save_config) =
                                    Config::<config::Game>::read_from_file_or_show_dialog(
                                        &config_path,
                                        &game_title,
                                    );
                                config.unwrap_or_else(move || {
                                    if save_config {
                                        Config {
                                            contents: config::Game::default(),
                                            dirty: true,
                                            path: Some(config_path),
                                        }
                                    } else {
                                        Config::default()
                                    }
                                })
                            };

                            match config::launch_config(
                                &state.global_config.contents,
                                &game_config.contents,
                                cart_header.as_ref(),
                                &game_title,
                            ) {
                                Ok(launch_config) => {
                                    state.game_config = Some(game_config);
                                    state.start(launch_config, rom, cart_info);
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
                        .build_with_ref(ui, &mut state.limit_framerate.value)
                    {
                        if state.limit_framerate.origin == config::SettingOrigin::Game {
                            let game_config = state.game_config.as_mut().unwrap();
                            game_config.contents.limit_framerate =
                                Some(state.limit_framerate.value);
                            game_config.dirty = true;
                        }
                        state.global_config.contents.limit_framerate = state.limit_framerate.value;
                        state.global_config.dirty = true;
                        if let Some(shared_state) = &state.emu_shared_state {
                            shared_state
                                .limit_framerate
                                .store(state.limit_framerate.value, Ordering::Relaxed);
                        }
                    }
                });
                #[cfg(feature = "debug-views")]
                state.debug_views.render_menu_bar(ui, window);
            });

            #[cfg(feature = "log")]
            if let Some((console, _)) = &mut state.imgui_log {
                let _window_padding = ui.push_style_var(imgui::StyleVar::WindowPadding([6.0; 2]));
                let _item_spacing = ui.push_style_var(imgui::StyleVar::ItemSpacing([0.0; 2]));
                console.render_window(ui, Some(window.mono_font));
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
