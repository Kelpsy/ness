pub mod info;
mod map;

use crate::utils::BoxedByteSlice;
use info::Info;
use map::Map;

pub struct Cart {
    rom: BoxedByteSlice,
    ram: BoxedByteSlice,
    ram_modified: bool,
    map: Map,
}

impl Cart {
    #[allow(clippy::single_match)]
    pub fn new(rom: BoxedByteSlice, ram: BoxedByteSlice, info: &Info) -> Option<Self> {
        let mut map = Map::new();
        for region in &info.rom_map {
            let mut size = region.size.unwrap_or(rom.len() as u32);
            let offset = map::mirror(region.offset, size);
            size -= offset;
            for addr_range in &region.address_ranges {
                map.map::<true, false>(
                    Some(Self::handle_rom_read),
                    None,
                    addr_range.banks,
                    addr_range.addrs,
                    offset,
                    size,
                    region.mask,
                );
            }
        }
        for region in &info.ram_map {
            let mut size = region.size.unwrap_or(info.ram_size);
            let offset = map::mirror(region.offset, size);
            size -= offset;
            for addr_range in &region.address_ranges {
                map.map::<true, true>(
                    Some(Self::handle_ram_read),
                    Some(Self::handle_ram_write),
                    addr_range.banks,
                    addr_range.addrs,
                    offset,
                    size,
                    region.mask,
                );
            }
        }
        Some(Cart {
            rom,
            ram,
            ram_modified: false,
            map,
        })
    }

    #[inline]
    pub fn rom(&self) -> &BoxedByteSlice {
        &self.rom
    }

    #[inline]
    pub fn ram(&self) -> &BoxedByteSlice {
        &self.ram
    }

    #[inline]
    pub fn modify_ram(&mut self, f: impl FnOnce(&mut BoxedByteSlice)) {
        f(&mut self.ram);
        self.ram_modified = true;
    }

    #[inline]
    pub fn ram_modified(&self) -> bool {
        self.ram_modified
    }

    #[inline]
    pub fn mark_ram_flushed(&mut self) {
        self.ram_modified = false;
    }

    #[inline]
    pub fn read_data(&mut self, addr: u32) -> Option<u8> {
        self.map
            .read_data(addr)
            .map(|(read, addr)| read(self, addr))
    }

    #[inline]
    pub fn write_data(&mut self, addr: u32, value: u8) -> Option<()> {
        self.map
            .write_data(addr)
            .map(|(write, addr)| write(self, addr, value))
    }

    fn handle_rom_read(&mut self, offset: u32) -> u8 {
        self.rom[offset as usize]
    }

    fn handle_ram_read(&mut self, offset: u32) -> u8 {
        self.ram[offset as usize]
    }

    fn handle_ram_write(&mut self, offset: u32, value: u8) {
        self.ram_modified = true;
        self.ram[offset as usize] = value;
    }
}
