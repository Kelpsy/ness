mod saves;

use super::{
    audio,
    utils::{config_base, data_base},
};
use ness_core::{
    cart::info::header::{Header as CartHeader, Region},
    Model,
};
use saves::{save_path, SavePathConfig};
use serde::{Deserialize, Serialize};
use std::{
    fmt, fs, io,
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LoggingKind {
    Imgui,
    Term,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModelConfig {
    Auto,
    Ntsc,
    Pal,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Global {
    pub model: ModelConfig,
    pub limit_framerate: bool,
    pub sync_to_audio: bool,
    pub audio_interp_method: audio::InterpMethod,
    pub pause_on_launch: bool,
    pub autosave_interval_ms: f32,
    pub save_dir_path: PathBuf,

    pub fullscreen_render: bool,
    pub cart_db_path: PathBuf,
    pub board_db_path: PathBuf,
    pub logging_kind: LoggingKind,
    pub window_size: (u32, u32),
    pub imgui_config_path: Option<PathBuf>,
}

impl Default for Global {
    fn default() -> Self {
        let config_base = config_base();
        let data_base = data_base();
        Global {
            model: ModelConfig::Auto,
            limit_framerate: true,
            sync_to_audio: true,
            audio_interp_method: audio::InterpMethod::Nearest,
            pause_on_launch: false,
            autosave_interval_ms: 1000.0,
            save_dir_path: data_base.join("saves"),

            fullscreen_render: true,
            cart_db_path: data_base.join("db/carts.bml"),
            board_db_path: data_base.join("db/boards.bml"),
            logging_kind: LoggingKind::Imgui,
            window_size: (1300, 800),
            imgui_config_path: Some(config_base.join("imgui.ini")),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Game {
    pub model: Option<ModelConfig>,
    pub limit_framerate: Option<bool>,
    pub sync_to_audio: Option<bool>,
    pub audio_interp_method: Option<audio::InterpMethod>,
    pub pause_on_launch: Option<bool>,
    pub autosave_interval_ms: Option<f32>,
    pub save_path: Option<SavePathConfig>,
}

impl Default for Game {
    fn default() -> Self {
        Game {
            model: None,
            limit_framerate: None,
            sync_to_audio: None,
            audio_interp_method: None,
            pause_on_launch: None,
            autosave_interval_ms: None,
            save_path: Some(SavePathConfig::GlobalSingle),
        }
    }
}

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Json(serde_json::Error),
}

#[derive(Clone, Debug, Default)]
pub struct Config<T> {
    pub contents: T,
    pub dirty: bool,
    pub path: Option<PathBuf>,
}

impl<T> Config<T> {
    pub fn read_from_file(path: PathBuf) -> Result<Option<Self>, Error>
    where
        T: for<'de> Deserialize<'de>,
    {
        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(err) => {
                if err.kind() == io::ErrorKind::NotFound {
                    return Ok(None);
                } else {
                    return Err(Error::Io(err));
                }
            }
        };
        match serde_json::from_str(&content) {
            Ok(result) => Ok(Some(Config {
                contents: result,
                // `serde` might have added some default values, so save at least once just in case
                dirty: true,
                path: Some(path),
            })),
            Err(err) => Err(Error::Json(err)),
        }
    }

    pub fn read_from_file_or_show_dialog(path: &Path, config_name: &str) -> (Option<Self>, bool)
    where
        T: for<'de> Deserialize<'de>,
    {
        let path_str = path.to_str().unwrap_or(config_name);
        match Self::read_from_file(path.to_path_buf()) {
            Ok(config) => (config, true),
            Err(err) => match err {
                Error::Io(err) => {
                    config_error!(
                        concat!(
                            "Couldn't read `{}`: {}\n\nThe default values will be used, new ",
                            "changes will not be saved.",
                        ),
                        path_str,
                        err,
                    );
                    (None, false)
                }
                Error::Json(err) => (
                    None,
                    config_error!(
                        yes_no,
                        concat!(
                            "Couldn't parse `{}`: {}\n\nOverwrite the existing configuration file ",
                            "with the default values?",
                        ),
                        path_str,
                        err,
                    ),
                ),
            },
        }
    }

    pub fn flush(&mut self) -> Result<(), Error>
    where
        T: Serialize,
    {
        if let Some(path) = &self.path {
            self.dirty = false;
            let content = serde_json::to_vec_pretty(&self.contents).map_err(Error::Json)?;
            fs::write(path, &content).map_err(Error::Io)
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingOrigin {
    Global,
    Game,
}

#[derive(Clone, Copy, Debug)]
pub struct RuntimeModifiable<T> {
    pub value: T,
    pub origin: SettingOrigin,
}

impl<T> RuntimeModifiable<T> {
    pub fn global(value: T) -> Self {
        RuntimeModifiable {
            value,
            origin: SettingOrigin::Global,
        }
    }
}

pub struct LaunchConfig {
    pub model: Model,
    pub limit_framerate: RuntimeModifiable<bool>,
    pub sync_to_audio: RuntimeModifiable<bool>,
    pub audio_interp_method: RuntimeModifiable<audio::InterpMethod>,
    pub pause_on_launch: bool,
    pub autosave_interval_ms: RuntimeModifiable<f32>,
    pub cur_save_path: Option<PathBuf>,
}

#[derive(Debug)]
pub enum LaunchConfigError {
    UnknownModel,
}

impl fmt::Display for LaunchConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LaunchConfigError::UnknownModel => {
                write!(
                    f,
                    concat!(
                        "Couldn't detect console model from the provided game, please specify one ",
                        "directly",
                    )
                )
            }
        }
    }
}

pub fn launch_config(
    global_config: &Global,
    game_config: &Game,
    header: Option<&CartHeader>,
    game_title: &str,
) -> Result<LaunchConfig, Vec<LaunchConfigError>> {
    macro_rules! plain_setting {
        ($field: ident) => {
            game_config.$field.unwrap_or(global_config.$field)
        };
    }

    macro_rules! runtime_modifiable {
        ($field: ident) => {
            if let Some(value) = game_config.$field {
                RuntimeModifiable {
                    value,
                    origin: SettingOrigin::Game,
                }
            } else {
                RuntimeModifiable {
                    value: global_config.$field,
                    origin: SettingOrigin::Global,
                }
            }
        };
    }

    let mut errors = Vec::new();

    // 00h -  International (eg. SGB)  (any)
    // 00h J  Japan                    (NTSC)
    // 01h E  USA and Canada           (NTSC)
    // 02h P  Europe, Oceania, Asia    (PAL)
    // 03h W  Sweden/Scandinavia       (PAL)
    // 04h -  Finland                  (PAL)
    // 05h -  Denmark                  (PAL)
    // 06h F  France                   (SECAM, PAL-like 50Hz)
    // 07h H  Holland                  (PAL)
    // 08h S  Spain                    (PAL)
    // 09h D  Germany, Austria, Switz  (PAL)
    // 0Ah I  Italy                    (PAL)
    // 0Bh C  China, Hong Kong         (PAL)
    // 0Ch -  Indonesia                (PAL)
    // 0Dh K  South Korea              (NTSC) (North Korea would be PAL)
    // 0Eh A  Common (?)               (?)
    // 0Fh N  Canada                   (NTSC)
    // 10h B  Brazil                   (PAL-M, NTSC-like 60Hz)
    // 11h U  Australia                (PAL)
    // 12h X  Other variation          (?)
    // 13h Y  Other variation          (?)
    // 14h Z  Other variation          (?)

    let model = match plain_setting!(model) {
        ModelConfig::Auto => {
            // TODO: Detect whether the game's region is PAL or NTSC (can be inferred from the
            // header in most cases)
            match header.as_ref().map(|h| h.region).unwrap_or(Region::Common) {
                // TODO: Japan is NTSC but "International" is not, figure out a way to discern them.
                Region::InternationalJapan
                | Region::UsaCanada
                | Region::SouthKorea
                | Region::Canada
                | Region::Brazil => Some(Model::Ntsc),
                Region::EuropeOceaniaAsia
                | Region::SwedenScandinavia
                | Region::Finland
                | Region::Denmark
                | Region::France
                | Region::Holland
                | Region::Spain
                | Region::GermanyAustriaSwitzerland
                | Region::Italy
                | Region::ChinaHongKong
                | Region::Indonesia
                | Region::Australia => Some(Model::Pal),
                Region::Common | Region::Unknown(_) => {
                    errors.push(LaunchConfigError::UnknownModel);
                    None
                }
            }
        }
        ModelConfig::Ntsc => Some(Model::Ntsc),
        ModelConfig::Pal => Some(Model::Pal),
    };

    if !errors.is_empty() {
        return Err(errors);
    }

    let limit_framerate = runtime_modifiable!(limit_framerate);
    let sync_to_audio = runtime_modifiable!(sync_to_audio);
    let audio_interp_method = runtime_modifiable!(audio_interp_method);
    let pause_on_launch = plain_setting!(pause_on_launch);
    let autosave_interval_ms = runtime_modifiable!(autosave_interval_ms);

    let cur_save_path = save_path(
        &global_config.save_dir_path,
        &game_config.save_path,
        game_title,
    );

    Ok(LaunchConfig {
        model: model.unwrap(),
        limit_framerate,
        sync_to_audio,
        audio_interp_method,
        pause_on_launch,
        autosave_interval_ms,
        cur_save_path,
    })
}
