mod header;

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

impl Info {
    pub fn guess(rom: ByteSlice) -> Option<Self> {
        if rom.len() < 0x8000 {
            return None;
        }

        let header = Header::new(
            ByteSlice::new(&rom[0x7FB0..0x8000]),
            header::BaseMapMode::LoRom,
        )
        .or_else(|| {
            rom[..].get(0xFFB0..0x1_0000).and_then(|header_bytes| {
                Header::new(ByteSlice::new(header_bytes), header::BaseMapMode::HiRom)
            })
        })
        .or_else(|| {
            rom[..].get(0x40_FFB0..0x41_0000).and_then(|header_bytes| {
                Header::new(ByteSlice::new(header_bytes), header::BaseMapMode::ExHiRom)
            })
        })?;

        let (rom_map, ram_map) = match header.map_mode.base() {
            header::BaseMapMode::LoRom => {
                let mut rom_ranges = vec![
                    MapAddrRange {
                        banks: (0x00, 0x7D),
                        addrs: (0x8000, 0xFFFF),
                    },
                    MapAddrRange {
                        banks: (0x80, 0xFF),
                        addrs: (0x8000, 0xFFFF),
                    },
                ];
                if header.ram_size == 0 {
                    rom_ranges.extend_from_slice(&[
                        MapAddrRange {
                            banks: (0x40, 0x7D),
                            addrs: (0x0000, 0x7FFF),
                        },
                        MapAddrRange {
                            banks: (0xC0, 0xFF),
                            addrs: (0x0000, 0x7FFF),
                        },
                    ]);
                }
                (
                    vec![MapRegion {
                        address_ranges: rom_ranges,
                        offset: 0,
                        size: None,
                        mask: 0x8000,
                    }],
                    // TODO: Same issue as HiROM, although only the exact ranges inside banks
                    // differ. In addition, ROM banks can be wildly different.
                    vec![],
                )
            }
            header::BaseMapMode::HiRom => (
                vec![MapRegion {
                    address_ranges: vec![
                        MapAddrRange {
                            banks: (0x00, 0x3F),
                            addrs: (0x8000, 0xFFFF),
                        },
                        MapAddrRange {
                            banks: (0x80, 0xBF),
                            addrs: (0x8000, 0xFFFF),
                        },
                        MapAddrRange {
                            banks: (0x40, 0x7D),
                            addrs: (0x0000, 0xFFFF),
                        },
                        MapAddrRange {
                            banks: (0xC0, 0xFF),
                            addrs: (0x0000, 0xFFFF),
                        },
                    ],
                    offset: 0,
                    size: None,
                    mask: 0,
                }],
                // TODO: Some HiROM boards put SRAM in banks 20-3F and A0-BF, others put them in
                // banks 10-1F, 30-3F, 90-9F and B0-BF, how to guess which one of those layouts
                // is needed?
                vec![],
            ),
            header::BaseMapMode::ExHiRom => (
                vec![
                    MapRegion {
                        address_ranges: vec![
                            MapAddrRange {
                                banks: (0x00, 0x3F),
                                addrs: (0x8000, 0xFFFF),
                            },
                            MapAddrRange {
                                banks: (0x40, 0x7D),
                                addrs: (0x0000, 0xFFFF),
                            },
                        ],
                        offset: 0x40_0000,
                        size: None,
                        mask: 0,
                    },
                    MapRegion {
                        address_ranges: vec![
                            MapAddrRange {
                                banks: (0x80, 0xBF),
                                addrs: (0x8000, 0xFFFF),
                            },
                            MapAddrRange {
                                banks: (0xC0, 0xFF),
                                addrs: (0x0000, 0xFFFF),
                            },
                        ],
                        offset: 0,
                        size: None,
                        mask: 0xC0_0000,
                    },
                ],
                if header.ram_size != 0 {
                    vec![MapRegion {
                        address_ranges: vec![MapAddrRange {
                            banks: (0x80, 0xBF),
                            addrs: (0x6000, 0x7FFF),
                        }],
                        offset: 0,
                        size: None,
                        mask: 0xE000,
                    }]
                } else {
                    vec![]
                },
            ),
        };

        Some(Info {
            title: header.title,
            ram_size: if header.chipset.has_ram {
                header.ram_size
            } else {
                0
            },
            has_battery: header.chipset.has_battery,
            rom_map,
            ram_map,
        })
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
