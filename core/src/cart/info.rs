pub mod db;
mod guess;
pub mod header;

use super::map;
use crate::utils::ByteSlice;
use header::Header;

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

pub type Map = Vec<MapRegion>;

#[derive(Debug)]
pub struct Info {
    pub title: Option<String>,
    pub ram_size: u32,
    pub has_battery: bool,
    pub rom_map: Map,
    pub ram_map: Map,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Db,
    Guess,
    Default,
}

impl Info {
    pub fn new(
        rom: ByteSlice,
        db_data: Option<(&db::Db, [u8; 32])>,
    ) -> (Self, Option<Header>, Source) {
        db_data
            .and_then(|(db, rom_hash)| {
                Self::from_db(db, &rom_hash).and_then(|info| {
                    for region in &info.rom_map {
                        for addr_range in &region.address_ranges {
                            if addr_range.banks.0 == 0
                                && addr_range.addrs.0 <= 0xFFB0
                                && addr_range.addrs.1 >= 0xFFDF
                            {
                                let size = region.size.unwrap_or(rom.len() as u32);
                                let base_offset = map::mirror(region.offset, size);
                                let offset = (base_offset
                                    + map::mirror(
                                        map::reduce(0xFF80, region.mask),
                                        size - base_offset,
                                    )) as usize;
                                return Header::new(
                                    ByteSlice::new(&rom[offset..offset + 0x30]),
                                    None,
                                )
                                .map(|header| (info, Some(header), Source::Db));
                            }
                        }
                    }
                    None
                })
            })
            .or_else(|| Self::guess(rom).map(|(info, header)| (info, Some(header), Source::Guess)))
            .unwrap_or_else(|| (Default::default(), None, Source::Default))
    }
}

impl Default for Info {
    fn default() -> Self {
        Info {
            title: None,
            ram_size: 0,
            has_battery: false,
            rom_map: vec![MapRegion {
                address_ranges: vec![
                    MapAddrRange {
                        banks: (0x00, 0x7D),
                        addrs: (0x8000, 0xFFFF),
                    },
                    MapAddrRange {
                        banks: (0x80, 0xFF),
                        addrs: (0x8000, 0xFFFF),
                    },
                    MapAddrRange {
                        banks: (0x40, 0x7D),
                        addrs: (0x0000, 0x7FFF),
                    },
                    MapAddrRange {
                        banks: (0xC0, 0xFF),
                        addrs: (0x0000, 0x7FFF),
                    },
                ],
                offset: 0,
                size: None,
                mask: 0x8000,
            }],
            ram_map: vec![],
        }
    }
}
