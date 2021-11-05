use super::bml;

use core::fmt::{self, Display};
use std::{borrow::Cow, collections::HashMap, error::Error};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoadError<'a> {
    Bml(bml::ParseError),
    UnexpectedRootNode(bml::Node<'a>),
    MissingCartAttr {
        name: &'static str,
    },
    MissingCartAttrValue {
        name: &'static str,
    },
    UnexpectedCartAttrAttrs {
        name: &'static str,
        attrs: Vec<bml::Node<'a>>,
    },
    UnexpectedCartAttrs(Vec<bml::Node<'a>>),
    InvalidSha256(Cow<'a, str>),
    UnexpectedHardware(bml::Node<'a>),
    MissingHardwareAttr {
        ty: &'a str,
        name: &'static str,
    },
    MissingHardwareAttrValue {
        ty: &'a str,
        name: &'static str,
    },
    UnexpectedHardwareAttrValue {
        ty: &'a str,
        name: &'static str,
        value: Cow<'a, str>,
    },
    UnexpectedHardwareAttrAttrs {
        ty: &'a str,
        name: &'static str,
        attrs: Vec<bml::Node<'a>>,
    },
    UnexpectedHardwareAttrs {
        ty: &'a str,
        attrs: Vec<bml::Node<'a>>,
    },
    UnknownMemoryType(Cow<'a, str>),
    UnknownMemoryContent {
        memory_ty: &'static str,
        content: Cow<'a, str>,
    },
    InvalidMemorySize(Cow<'a, str>),
    InvalidOscillatorFrequency(Cow<'a, str>),
}

impl<'a> Error for LoadError<'a> {}

impl<'a> Display for LoadError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Self::Bml(err) = self {
            err.fmt(f)
        } else {
            f.write_str(match self {
                Self::Bml(_) => unreachable!(),
                Self::UnexpectedRootNode(_) => "Unexpected carts root node",
                Self::MissingCartAttr { .. } => "Missing required cart attribute",
                Self::MissingCartAttrValue { .. } => "Missing required cart attribute value",
                Self::UnexpectedCartAttrAttrs { .. } => "Unexpected cart attribute sub-attribute",
                Self::UnexpectedCartAttrs { .. } => "Unexpected cart attribute",
                Self::InvalidSha256(_) => "Invalid cart SHA-256 hash",
                Self::UnexpectedHardware(_) => "Unexpected cart hardware",
                Self::MissingHardwareAttr { .. } => "Missing required cart hardware attribute",
                Self::MissingHardwareAttrValue { .. } => {
                    "Missing required cart hardware attribute value"
                }
                Self::UnexpectedHardwareAttrValue { .. } => {
                    "Unexpected cart hardware attribute value"
                }
                Self::UnexpectedHardwareAttrAttrs { .. } => {
                    "Unexpected cart hardware attribute sub-attribute"
                }
                Self::UnexpectedHardwareAttrs { .. } => "Unexpected cart hardware attribute",
                Self::UnknownMemoryType(_) => "Unknown cart memory type",
                Self::UnknownMemoryContent { .. } => "Unknown cart memory content",
                Self::InvalidMemorySize(_) => "Invalid cart memory size",
                Self::InvalidOscillatorFrequency(_) => "Invalid cart oscillator frequency",
            })
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RomContent {
    Program,
    Boot,
    Data,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RamContent {
    Save,
    Internal,
    Data,
    Download,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rom {
    pub size: u32,
    pub content: RomContent,
    pub manufacturer: Option<String>,
    pub architecture: Option<String>,
    pub identifier: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ram {
    pub size: u32,
    pub content: RamContent,
    pub volatile: bool,
    pub manufacturer: Option<String>,
    pub architecture: Option<String>,
    pub identifier: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rtc {
    pub size: u32,
    pub manufacturer: Option<String>,
    pub architecture: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Oscillator {
    pub frequency: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Hardware {
    Rom(Rom),
    Ram(Ram),
    Rtc(Rtc),
    Oscillator(Oscillator),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub label: String,
    pub name: String,
    pub region: String,
    pub revision: String,
    pub board: String,
    pub hardware: Vec<Hardware>,
}

pub type Db = HashMap<[u8; 32], Entry>;

pub fn load(input: &str) -> Result<Db, LoadError> {
    let mut result = HashMap::new();
    for mut node in bml::parse(input).map_err(LoadError::Bml)? {
        if node.name == "database" {
            continue;
        }
        if node.name != "game" || node.value.is_some() {
            return Err(LoadError::UnexpectedRootNode(node));
        }
        let board_node = node
            .remove_attr("board")
            .ok_or(LoadError::MissingCartAttr { name: "board" })?;
        let mut result_hardware = vec![];
        for mut hardware in board_node.attrs {
            macro_rules! remove_value_attr {
                (opt $ty: expr, $name: expr) => {
                    match hardware.remove_value_attr($name) {
                        Ok(value) => Some(value),
                        Err(bml::ValueAttrError::Missing) => None,
                        Err(bml::ValueAttrError::MissingValue) => {
                            return Err(LoadError::MissingHardwareAttrValue {
                                ty: $ty,
                                name: $name,
                            })
                        }
                        Err(bml::ValueAttrError::UnexpectedAttrs(attrs)) => {
                            return Err(LoadError::UnexpectedHardwareAttrAttrs {
                                ty: $ty,
                                name: $name,
                                attrs,
                            })
                        }
                    }
                };
                ($ty: expr, $name: expr) => {
                    hardware.remove_value_attr($name).map_err(|err| match err {
                        bml::ValueAttrError::Missing => LoadError::MissingHardwareAttr {
                            ty: $ty,
                            name: $name,
                        },
                        bml::ValueAttrError::MissingValue => LoadError::MissingHardwareAttrValue {
                            ty: $ty,
                            name: $name,
                        },
                        bml::ValueAttrError::UnexpectedAttrs(attrs) => {
                            LoadError::UnexpectedHardwareAttrAttrs {
                                ty: $ty,
                                name: $name,
                                attrs,
                            }
                        }
                    })?
                };
            }

            if hardware.value.is_some() {
                return Err(LoadError::UnexpectedHardware(hardware));
            }
            match hardware.name {
                "memory" => {
                    let ty = remove_value_attr!("memory", "type");
                    let size = {
                        let value = remove_value_attr!("memory", "size");
                        parse_hex!(u32, value.as_ref(), LoadError::InvalidMemorySize(value))
                    };
                    let content = remove_value_attr!("memory", "content");

                    let manufacturer =
                        remove_value_attr!(opt "memory", "manufacturer").map(Cow::into_owned);
                    let architecture =
                        remove_value_attr!(opt "memory", "architecture").map(Cow::into_owned);
                    let identifier =
                        remove_value_attr!(opt "memory", "identifier").map(Cow::into_owned);

                    result_hardware.push(match ty.as_ref() {
                        "ROM" => Hardware::Rom(Rom {
                            size,
                            content: match content.as_ref() {
                                "Program" => RomContent::Program,
                                "Boot" => RomContent::Boot,
                                "Data" => RomContent::Data,
                                _ => {
                                    return Err(LoadError::UnknownMemoryContent {
                                        memory_ty: "ROM",
                                        content,
                                    })
                                }
                            },
                            manufacturer,
                            architecture,
                            identifier,
                        }),

                        "RAM" => Hardware::Ram(Ram {
                            size,
                            content: match content.as_ref() {
                                "Save" => RamContent::Save,
                                "Internal" => RamContent::Internal,
                                "Data" => RamContent::Data,
                                "Download" => RamContent::Download,
                                _ => {
                                    return Err(LoadError::UnknownMemoryContent {
                                        memory_ty: "RAM",
                                        content,
                                    })
                                }
                            },
                            manufacturer,
                            architecture,
                            identifier,
                            volatile: hardware.remove_marker("volatile").map_err(
                                |err| match err {
                                    bml::MarkerAttrError::UnexpectedValue(value) => {
                                        LoadError::UnexpectedHardwareAttrValue {
                                            ty: "memory",
                                            name: "volatile",
                                            value,
                                        }
                                    }
                                    bml::MarkerAttrError::UnexpectedAttrs(attrs) => {
                                        LoadError::UnexpectedHardwareAttrAttrs {
                                            ty: "memory",
                                            name: "volatile",
                                            attrs,
                                        }
                                    }
                                },
                            )?,
                        }),

                        "RTC" => {
                            if content != "Time" {
                                return Err(LoadError::UnknownMemoryContent {
                                    memory_ty: "RTC",
                                    content,
                                });
                            }
                            Hardware::Rtc(Rtc {
                                size,
                                manufacturer,
                                architecture,
                            })
                        }

                        _ => return Err(LoadError::UnknownMemoryType(ty)),
                    });
                }

                "oscillator" => {
                    let frequency = {
                        let value = remove_value_attr!("oscillator", "frequency");
                        value
                            .parse()
                            .map_err(|_| LoadError::InvalidOscillatorFrequency(value))?
                    };
                    result_hardware.push(Hardware::Oscillator(Oscillator { frequency }));
                }
                _ => return Err(LoadError::UnexpectedHardware(hardware)),
            }

            if !hardware.attrs.is_empty() {
                return Err(LoadError::UnexpectedHardwareAttrs {
                    ty: hardware.name,
                    attrs: hardware.attrs,
                });
            }
        }

        macro_rules! remove_value_attr {
            (opt $name: expr) => {
                match node.remove_value_attr($name) {
                    Ok(value) => Some(value),
                    Err(bml::ValueAttrError::Missing) => None,
                    Err(bml::ValueAttrError::MissingValue) => {
                        return Err(LoadError::MissingCartAttrValue { name: $name })
                    }
                    Err(bml::ValueAttrError::UnexpectedAttrs(attrs)) => {
                        return Err(LoadError::UnexpectedCartAttrAttrs { name: $name, attrs });
                    }
                }
            };
            ($name: expr) => {
                node.remove_value_attr($name).map_err(|err| match err {
                    bml::ValueAttrError::Missing => LoadError::MissingCartAttr { name: $name },
                    bml::ValueAttrError::MissingValue => {
                        LoadError::MissingCartAttrValue { name: $name }
                    }
                    bml::ValueAttrError::UnexpectedAttrs(attrs) => {
                        LoadError::UnexpectedCartAttrAttrs { name: $name, attrs }
                    }
                })?
            };
        }

        let sha256 = {
            let value = remove_value_attr!("sha256");
            let mut output = [0; 32];
            for (i, byte) in output.iter_mut().enumerate() {
                *byte = match value
                    .get(i * 2..i * 2 + 2)
                    .ok_or(())
                    .and_then(|value| u8::from_str_radix(value, 16).map_err(drop))
                {
                    Ok(value) => value,
                    Err(_) => return Err(LoadError::InvalidSha256(value)),
                };
            }
            output
        };
        let label = remove_value_attr!("label").into_owned();
        let name = remove_value_attr!("name").into_owned();
        let region = remove_value_attr!("region").into_owned();
        let revision = remove_value_attr!("revision").into_owned();

        let _note = remove_value_attr!(opt "note");

        if !node.attrs.is_empty() {
            return Err(LoadError::UnexpectedCartAttrs(node.attrs));
        }

        result.insert(
            sha256,
            Entry {
                label,
                name,
                region,
                revision,
                board: board_node
                    .value
                    .ok_or(LoadError::MissingCartAttrValue { name: "board" })?
                    .into_owned(),
                hardware: result_hardware,
            },
        );
    }
    Ok(result)
}
