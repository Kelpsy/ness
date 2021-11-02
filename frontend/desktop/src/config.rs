mod saves;

use ness_core::Model;
use saves::{save_path, SavePathConfig};
use serde::{Deserialize, Serialize};
use std::{
    env, fmt, fs, io,
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LoggingKind {
    Imgui,
    Term,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", untagged)]
pub enum ModelConfig {
    Auto,
    Manual(Model),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Global {
    pub model: ModelConfig,
    pub limit_framerate: bool,
    pub save_dir_path: PathBuf,
    pub logging_kind: LoggingKind,
    pub window_size: (u32, u32),
}

impl Default for Global {
    fn default() -> Self {
        Global {
            model: ModelConfig::Auto,
            limit_framerate: true,
            save_dir_path: match env::var_os("XDG_DATA_HOME") {
                Some(data_dir) => Path::new(&data_dir).join("ness"),
                None => home::home_dir()
                    .map(|home| home.join(".local/share/ness"))
                    .unwrap_or_else(|| PathBuf::from("/.local/share/ness")),
            }
            .join("saves"),
            logging_kind: LoggingKind::Imgui,
            window_size: (1300, 800),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Game {
    pub model: Option<ModelConfig>,
    pub limit_framerate: Option<bool>,
    pub save_path: Option<SavePathConfig>,
}

impl Default for Game {
    fn default() -> Self {
        Game {
            model: None,
            limit_framerate: None,
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

pub struct LaunchConfig {
    pub model: Model,
    pub limit_framerate: RuntimeModifiable<bool>,
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

    let model = match plain_setting!(model) {
        ModelConfig::Auto => {
            errors.push(LaunchConfigError::UnknownModel);
            None
        }
        ModelConfig::Manual(model) => Some(model),
    };

    if !errors.is_empty() {
        return Err(errors);
    }

    let limit_framerate = runtime_modifiable!(limit_framerate);

    let cur_save_path = save_path(
        &global_config.save_dir_path,
        &game_config.save_path,
        game_title,
    );

    Ok(LaunchConfig {
        model: model.unwrap(),
        limit_framerate,
        cur_save_path,
    })
}
