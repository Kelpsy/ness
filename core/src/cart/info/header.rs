use crate::utils::ByteSlice;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MakerCode {
    Old(u8),
    New(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Region {
    InternationalJapan,
    UsaCanada,
    EuropeOceaniaAsia,
    SwedenScandinavia,
    Finland,
    Denmark,
    France,
    Holland,
    Spain,
    GermanyAustriaSwitzerland,
    Italy,
    ChinaHongKong,
    Indonesia,
    SouthKorea,
    Common,
    Canada,
    Brazil,
    Australia,
    Unknown(u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Coprocessor {
    Dsp,
    Gsu,
    Obc1,
    Sa1,
    SDd1,
    SRtc,
    Other,
    Spc7110,
    St010St011,
    St018,
    Cx4,
    Unknown(u8),
    UnknownCustom(u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Chipset {
    pub coprocessor: Coprocessor,
    pub has_ram: bool,
    pub has_battery: bool,
    pub has_rtc: bool,
}

#[allow(clippy::enum_variant_names)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BaseMapMode {
    LoRom,
    HiRom,
    ExHiRom,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapMode {
    LoRom,
    HiRom,
    LoRomSdd1,
    LoRomSa1,
    ExHiRom,
    HiRomSpc7110,
}

impl MapMode {
    #[inline]
    pub fn base(self) -> BaseMapMode {
        match self {
            Self::LoRom | Self::LoRomSdd1 | Self::LoRomSa1 => BaseMapMode::LoRom,
            Self::HiRom => BaseMapMode::HiRom,
            Self::ExHiRom | Self::HiRomSpc7110 => BaseMapMode::ExHiRom,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Header {
    pub title: Option<String>,
    pub maker_code: Option<MakerCode>,
    pub game_code: Option<String>,
    pub region: Region,
    pub version: u8,
    pub special_version: u8,

    pub chipset: Chipset,
    pub map_mode: MapMode,
    pub fast_rom: bool,
    pub rom_size: u32,
    pub ram_size: u32,
    pub expansion_flash_size: u32,
    pub expansion_ram_size: u32,
}

impl Header {
    pub fn new(bytes: ByteSlice, expected_base_map_mode: Option<BaseMapMode>) -> Option<Self> {
        let mut raw_chipset_sub_type = None;

        let mut raw_title = &bytes[0x10..0x25];
        if raw_title[0x14] == 0 {
            // Early extended header
            raw_title = &raw_title[..0x14];
            if bytes[0..0x10].iter().any(|byte| *byte != 0) {
                return None;
            }
            raw_chipset_sub_type = Some(bytes[0xF]);
        }

        if let Some(i) = raw_title.iter().position(|char| *char == 0) {
            raw_title = &raw_title[..i];
        }

        let title = {
            let title_str = core::str::from_utf8(raw_title).ok()?.trim_end();
            if title_str.is_empty() {
                None
            } else {
                Some(title_str.to_string())
            }
        };

        let region: Region = match bytes[0x29] {
            0x00 => Region::InternationalJapan,
            0x01 => Region::UsaCanada,
            0x02 => Region::EuropeOceaniaAsia,
            0x03 => Region::SwedenScandinavia,
            0x04 => Region::Finland,
            0x05 => Region::Denmark,
            0x06 => Region::France,
            0x07 => Region::Holland,
            0x08 => Region::Spain,
            0x09 => Region::GermanyAustriaSwitzerland,
            0x0A => Region::Italy,
            0x0B => Region::ChinaHongKong,
            0x0C => Region::Indonesia,
            0x0D => Region::SouthKorea,
            0x0E => Region::Common,
            0x0F => Region::Canada,
            0x10 => Region::Brazil,
            0x11 => Region::Australia,
            other => Region::Unknown(other),
        };

        let version = bytes[0x2B];

        let mut game_code = None;
        let mut expansion_flash_size = 0;
        let mut expansion_ram_size = 0;
        let mut special_version = 0;

        let raw_old_maker_code = bytes[0x2A];
        let maker_code = match raw_old_maker_code {
            0x33 => {
                game_code = Some(
                    core::str::from_utf8(&bytes[2..6])
                        .ok()?
                        .trim_end()
                        .to_string(),
                );
                expansion_flash_size = 0x400 << bytes[0xC];
                expansion_ram_size = 0x400 << bytes[0xD];
                special_version = bytes[0xE];
                raw_chipset_sub_type = Some(bytes[0xF]);
                Some(MakerCode::New(
                    core::str::from_utf8(&bytes[0..2]).ok()?.to_string(),
                ))
            }
            0 => None,
            _ => Some(MakerCode::Old(raw_old_maker_code)),
        };

        let raw_chipset_type = bytes[0x26];
        let coprocessor = match raw_chipset_type >> 4 {
            0 if raw_chipset_type >= 3 => Coprocessor::Dsp,
            1 => Coprocessor::Gsu,
            2 => Coprocessor::Obc1,
            3 => Coprocessor::Sa1,
            4 => Coprocessor::SDd1,
            5 => Coprocessor::SRtc,
            0xE => Coprocessor::Other,
            0xF => match raw_chipset_sub_type? {
                0 => Coprocessor::Spc7110,
                1 => Coprocessor::St010St011,
                2 => Coprocessor::St018,
                0x10 => Coprocessor::Cx4,
                other => Coprocessor::UnknownCustom(other),
            },
            _ => Coprocessor::Unknown(raw_chipset_type),
        };
        let (has_ram, has_battery, has_rtc) = match raw_chipset_type & 0xF {
            0 | 3 => (false, false, false),
            1 | 4 => (true, false, false),
            2 | 5 | 0xA => (true, true, false),
            6 => (false, true, false),
            9 => (true, true, true),
            _ => return None,
        };
        let chipset = Chipset {
            coprocessor,
            has_ram,
            has_battery,
            has_rtc,
        };

        let rom_makeup = bytes[0x25];
        let map_mode = match rom_makeup & 0xF {
            0 => MapMode::LoRom,
            1 => MapMode::HiRom,
            2 => MapMode::LoRomSdd1,
            3 => MapMode::LoRomSa1,
            5 => MapMode::ExHiRom,
            0xA => MapMode::HiRomSpc7110,
            _ => return None,
        };
        if let Some(expected_base_map_mode) = expected_base_map_mode {
            if map_mode.base() != expected_base_map_mode {
                return None;
            }
        }
        let fast_rom = rom_makeup & 0x10 != 0;

        let rom_size = 0x400 << bytes[0x27];
        let ram_size = 0x400 << bytes[0x28];

        Some(Header {
            title,
            maker_code,
            game_code,
            region,
            version,
            special_version,

            chipset,
            map_mode,
            fast_rom,
            rom_size,
            ram_size,
            expansion_flash_size,
            expansion_ram_size,
        })
    }
}
