// Needed in some places as otherwise the compiler will complain about moving values, as it can't
// detect the immediate return in an `.ok_or(Err(moved_value))?`
macro_rules! unwrap_or_err {
    ($value: expr, $err: expr) => {
        if let Some(value) = $value {
            value
        } else {
            return Err($err);
        }
    };
}

macro_rules! parse_hex {
    ($ty: ty, $str: expr, $err: expr) => {
        if let Ok(value) = <$ty>::from_str_radix($str.trim_start_matches("0x"), 16) {
            value
        } else {
            return Err($err);
        }
    };
}

pub mod bml;
mod boards;
pub use boards::LoadError as BoardsLoadError;
mod carts;
pub use carts::LoadError as CartsLoadError;

use super::info::{self, Info};
use core::fmt::{self, Display};
use std::error::Error;

pub struct Db {
    carts: carts::Db,
    boards: boards::Db,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoadError<'a> {
    Carts(CartsLoadError<'a>),
    Boards(BoardsLoadError<'a>),
}

impl<'a> Error for LoadError<'a> {}

impl<'a> Display for LoadError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Carts(err) => err.fmt(f),
            Self::Boards(err) => err.fmt(f),
        }
    }
}

impl Db {
    pub fn load<'a>(carts_db: &'a str, boards_db: &'a str) -> Result<Self, LoadError<'a>> {
        Ok(Db {
            carts: carts::load(carts_db).map_err(LoadError::Carts)?,
            boards: boards::load(boards_db).map_err(LoadError::Boards)?,
        })
    }

    pub fn cart_info(&self, hash: &[u8; 32]) -> Option<Info> {
        let cart = self.carts.get(hash)?;
        let board = self.boards.get(&cart.board)?;

        let mut rom_map = vec![];
        let mut ram_map = vec![];
        for hardware in board.iter() {
            match hardware {
                boards::Hardware::Rom {
                    content: boards::RomContent::Program,
                    map: db_map,
                } => {
                    rom_map.extend(db_map.iter().map(|db_map_region| {
                        info::MapRegion {
                            address_ranges: db_map_region
                                .address_ranges
                                .iter()
                                .map(|db_addr_range| info::MapAddrRange {
                                    addrs: db_addr_range.addrs,
                                    banks: db_addr_range.banks,
                                })
                                .collect(),
                            offset: db_map_region.offset,
                            size: db_map_region.size,
                            mask: db_map_region.mask,
                        }
                    }));
                }
                boards::Hardware::Ram {
                    content: boards::RamContent::Save,
                    map: db_map,
                } => {
                    ram_map.extend(db_map.iter().map(|db_map_region| {
                        info::MapRegion {
                            address_ranges: db_map_region
                                .address_ranges
                                .iter()
                                .map(|db_addr_range| info::MapAddrRange {
                                    addrs: db_addr_range.addrs,
                                    banks: db_addr_range.banks,
                                })
                                .collect(),
                            offset: db_map_region.offset,
                            size: db_map_region.size,
                            mask: db_map_region.mask,
                        }
                    }));
                }
                _ => {}
            }
        }

        let save_ram_size = cart
            .hardware
            .iter()
            .find_map(|hardware| match hardware {
                carts::Hardware::Ram(carts::Ram {
                    size,
                    content: carts::RamContent::Save,
                    ..
                }) => Some(*size as u32),
                _ => None,
            })
            .unwrap_or(0);

        Some(Info {
            title: Some(cart.name.clone()),
            ram_size: save_ram_size,
            has_battery: save_ram_size != 0,
            rom_map,
            ram_map,
        })
    }
}
