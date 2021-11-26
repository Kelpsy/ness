use crate::utils::zeroed_box;

pub struct Timings {
    values: Box<[u8; Self::ENTRIES]>,
    fastrom_enabled: bool,
}

impl Timings {
    pub const PAGE_SIZE_SHIFT: usize = 9;
    pub const PAGE_SIZE: usize = 1 << Self::PAGE_SIZE_SHIFT;
    pub const ENTRIES: usize = 1 << (24 - Self::PAGE_SIZE_SHIFT);

    pub(crate) fn new() -> Self {
        let mut result = Timings {
            values: zeroed_box(),
            fastrom_enabled: false,
        };
        for &banks in &[(0x00, 0x3F), (0x80, 0xBF)] {
            result.set(banks, (0x0000, 0x1FFF), 8);
            result.set(banks, (0x2000, 0x3FFF), 6);
            result.set(banks, (0x4000, 0x41FF), 12);
            result.set(banks, (0x4200, 0x5FFF), 6);
            result.set(banks, (0x6000, 0xFFFF), 8);
        }
        for &banks in &[(0x40, 0x7F), (0xC0, 0xFF)] {
            result.set(banks, (0x0000, 0xFFFF), 8);
        }
        result
    }

    pub(crate) fn get(&self, addr: u32) -> u8 {
        self.values[addr as usize >> Self::PAGE_SIZE_SHIFT & (Self::ENTRIES - 1)]
    }

    pub(crate) fn set(&mut self, bank_range: (u8, u8), addr_range: (u16, u16), cycles: u8) {
        assert!(
            addr_range.0 & (Self::PAGE_SIZE - 1) as u16 == 0,
            "Timing map address range start must be aligned to the page size"
        );
        assert!(
            addr_range.1 & (Self::PAGE_SIZE - 1) as u16 == (Self::PAGE_SIZE - 1) as u16,
            "Timing map address range end must be aligned to the page size"
        );
        for bank_base in
            (((bank_range.0 as usize) << 16)..=((bank_range.1 as usize) << 16)).step_by(1 << 16)
        {
            self.values[(bank_base | addr_range.0 as usize) >> Self::PAGE_SIZE_SHIFT
                ..=(bank_base | addr_range.1 as usize) >> Self::PAGE_SIZE_SHIFT]
                .fill(cycles);
        }
    }

    #[inline]
    pub fn fastrom_enabled(&self) -> bool {
        self.fastrom_enabled
    }

    pub fn set_fastrom_enabled(&mut self, value: bool) {
        if value == self.fastrom_enabled {
            return;
        }
        self.fastrom_enabled = value;
        let cycles = if value { 6 } else { 8 };
        self.set((0x80, 0xBF), (0x8000, 0xFFFF), cycles);
        self.set((0xC0, 0xFF), (0x0000, 0xFFFF), cycles);
    }
}
