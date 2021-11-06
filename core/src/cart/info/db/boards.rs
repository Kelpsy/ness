use super::bml;

use core::fmt::{self, Display};
use std::{borrow::Cow, collections::HashMap, error::Error};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoadError<'a> {
    Bml(bml::ParseError),
    UnexpectedRootNode(bml::Node<'a>),
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
    MissingMapAttr {
        name: &'static str,
    },
    MissingMapAttrValue {
        name: &'static str,
    },
    UnexpectedMapAttrAttrs {
        name: &'static str,
        attrs: Vec<bml::Node<'a>>,
    },
    InvalidAddress(Cow<'a, str>),
    InvalidMapMask(Cow<'a, str>),
    InvalidMapOffset(Cow<'a, str>),
    InvalidMapSize(Cow<'a, str>),
}

impl<'a> Error for LoadError<'a> {}

impl<'a> Display for LoadError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Self::Bml(err) = self {
            err.fmt(f)
        } else {
            f.write_str(match self {
                Self::Bml(_) => unreachable!(),
                Self::UnexpectedRootNode(_) => "Unexpected boards root node",
                Self::UnexpectedHardware(_) => "Unexpected board hardware",
                Self::MissingHardwareAttr { .. } => "Missing required board hardware attribute",
                Self::MissingHardwareAttrValue { .. } => {
                    "Missing required board hardware attribute value"
                }
                Self::UnexpectedHardwareAttrValue { .. } => {
                    "Unexpected board hardware attribute value"
                }
                Self::UnexpectedHardwareAttrAttrs { .. } => {
                    "Unexpected board hardware attribute sub-attribute"
                }
                Self::UnexpectedHardwareAttrs { .. } => "Unexpected board hardware attribute",
                Self::UnknownMemoryType(_) => "Unknown board memory type",
                Self::UnknownMemoryContent { .. } => "Unknown board memory content",
                Self::MissingMapAttr { .. } => "Missing board memory map required attribute",
                Self::MissingMapAttrValue { .. } => {
                    "Missing board memory map required attribute value"
                }
                Self::UnexpectedMapAttrAttrs { .. } => {
                    "Unexpected board memory map attribute sub-attribute"
                }
                Self::InvalidAddress(_) => "Invalid board memory map address",
                Self::InvalidMapMask(_) => "Invalid board memory map mask",
                Self::InvalidMapOffset(_) => "Invalid board memory map offset",
                Self::InvalidMapSize(_) => "Invalid board memory map size",
            })
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RomContent {
    Program,
    Boot,
    Data,
    Expansion,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RamContent {
    Save,
    Internal,
    Data,
    Download,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapAddrRange {
    pub banks: (u8, u8),
    pub addrs: (u16, u16),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapRegion {
    pub address_ranges: Vec<MapAddrRange>,
    pub offset: u32,
    pub size: Option<u32>,
    pub mask: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Hardware {
    Rom {
        content: RomContent,
        map: Vec<MapRegion>,
    },
    Ram {
        content: RamContent,
        map: Vec<MapRegion>,
    },
    // TODO: External slots and processors
    Slot,
    Processor,
    Rtc,
}

pub type Entry = Vec<Hardware>;

pub type Db = HashMap<String, Entry>;

pub fn load(input: &str) -> Result<Db, LoadError> {
    let mut result = HashMap::new();
    for mut node in bml::parse(input).map_err(LoadError::Bml)? {
        if node.name == "database" {
            continue;
        }

        if node.name != "board" {
            return Err(LoadError::UnexpectedRootNode(node));
        }

        let name_pattern = match node.value.take() {
            Some(value) => value,
            None => return Err(LoadError::UnexpectedRootNode(node)),
        };
        let mut names = if let Some((start, variants, end)) =
            name_pattern.split_once('(').and_then(|(start, other)| {
                other
                    .split_once(')')
                    .map(|(variants, end)| (start, variants, end))
            }) {
            variants
                .split(',')
                .map(|variant| format!("{}{}{}", start, variant, end))
                .collect::<Vec<_>>()
        } else {
            vec![name_pattern.into_owned()]
        };

        let mut result_hardware = Vec::new();
        for mut hardware in node.attrs {
            macro_rules! remove_value_attr {
                (opt $ty: expr, $name: expr) => {
                    match hardware.remove_value_attr($name) {
                        Ok(value) => Some(value),
                        Err(bml::ValueAttrError::Missing) => None,
                        Err(bml::ValueAttrError::MissingValue) => {
                            return Err(Error::MissingHardwareAttrValue {
                                ty: $ty,
                                name: $name,
                            })
                        }
                        Err(bml::ValueAttrError::UnexpectedAttrs(attrs)) => {
                            return Err(Error::UnexpectedHardwareAttrAttrs {
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
                    let content = remove_value_attr!("memory", "content");

                    let mut result_map = vec![];
                    for mut map in hardware.attrs.drain_filter(|attr| attr.name == "map") {
                        macro_rules! remove_value_attr {
                            (opt $name: expr) => {
                                match map.remove_value_attr($name) {
                                    Ok(value) => Some(value),
                                    Err(bml::ValueAttrError::Missing) => None,
                                    Err(bml::ValueAttrError::MissingValue) => {
                                        return Err(LoadError::MissingMapAttrValue { name: $name })
                                    }
                                    Err(bml::ValueAttrError::UnexpectedAttrs(attrs)) => {
                                        return Err(LoadError::UnexpectedMapAttrAttrs {
                                            name: $name,
                                            attrs,
                                        })
                                    }
                                }
                            };
                            ($name: expr) => {
                                map.remove_value_attr($name).map_err(|err| match err {
                                    bml::ValueAttrError::Missing => {
                                        LoadError::MissingMapAttr { name: $name }
                                    }
                                    bml::ValueAttrError::MissingValue => {
                                        LoadError::MissingMapAttrValue { name: $name }
                                    }
                                    bml::ValueAttrError::UnexpectedAttrs(attrs) => {
                                        LoadError::UnexpectedMapAttrAttrs { name: $name, attrs }
                                    }
                                })?
                            };
                        }

                        let addr_ranges = remove_value_attr!("address");
                        let mask = remove_value_attr!(opt "mask").unwrap_or(Cow::Borrowed("0"));
                        let offset = remove_value_attr!(opt "base").unwrap_or(Cow::Borrowed("0"));
                        let size = remove_value_attr!(opt "size");

                        result_map.push(MapRegion {
                            address_ranges: {
                                let (bank_ranges, address_range) = unwrap_or_err!(
                                    addr_ranges.split_once(':'),
                                    LoadError::InvalidAddress(addr_ranges)
                                );

                                let mut result_addr_ranges = vec![];

                                let addrs = {
                                    let (start_address, end_address) = unwrap_or_err!(
                                        address_range.split_once('-'),
                                        LoadError::InvalidAddress(addr_ranges)
                                    );
                                    (
                                        parse_hex!(
                                            u16,
                                            start_address,
                                            LoadError::InvalidAddress(addr_ranges)
                                        ),
                                        parse_hex!(
                                            u16,
                                            end_address,
                                            LoadError::InvalidAddress(addr_ranges)
                                        ),
                                    )
                                };

                                for bank_range in bank_ranges.split(',') {
                                    let banks = {
                                        let (start_bank, end_bank) = unwrap_or_err!(
                                            bank_range.split_once('-'),
                                            LoadError::InvalidAddress(addr_ranges)
                                        );
                                        (
                                            parse_hex!(
                                                u8,
                                                start_bank,
                                                LoadError::InvalidAddress(addr_ranges)
                                            ),
                                            parse_hex!(
                                                u8,
                                                end_bank,
                                                LoadError::InvalidAddress(addr_ranges)
                                            ),
                                        )
                                    };
                                    result_addr_ranges.push(MapAddrRange { banks, addrs })
                                }

                                result_addr_ranges
                            },
                            offset: parse_hex!(u32, offset, LoadError::InvalidMapOffset(offset)),
                            size: match size {
                                Some(size) => {
                                    Some(parse_hex!(u32, size, LoadError::InvalidMapSize(size)))
                                }
                                None => None,
                            },
                            mask: parse_hex!(u32, mask, LoadError::InvalidMapMask(mask)),
                        });
                    }

                    result_hardware.push(match ty.as_ref() {
                        "ROM" => Hardware::Rom {
                            content: match content.as_ref() {
                                "Program" => RomContent::Program,
                                "Boot" => RomContent::Boot,
                                "Data" => RomContent::Data,
                                "Expansion" => RomContent::Expansion,
                                _ => {
                                    return Err(LoadError::UnknownMemoryContent {
                                        memory_ty: "ROM",
                                        content,
                                    })
                                }
                            },
                            map: result_map,
                        },

                        "RAM" => Hardware::Ram {
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
                            map: result_map,
                        },

                        _ => return Err(LoadError::UnknownMemoryType(ty)),
                    });

                    if !hardware.attrs.is_empty() {
                        return Err(LoadError::UnexpectedHardwareAttrs {
                            ty: "memory",
                            attrs: hardware.attrs,
                        });
                    }
                }

                // TODO: External slots and processors
                "slot" => {
                    result_hardware.push(Hardware::Slot);
                }

                "processor" => {
                    result_hardware.push(Hardware::Processor);
                }

                "rtc" => {
                    result_hardware.push(Hardware::Rtc);
                }

                _ => return Err(LoadError::UnexpectedHardware(hardware)),
            }
        }

        for name in names.drain(..names.len().saturating_sub(1)) {
            result.insert(name, result_hardware.clone());
        }
        if let Some(name) = names.pop() {
            result.insert(name, result_hardware);
        }
    }
    Ok(result)
}
