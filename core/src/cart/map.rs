use super::Cart;
use crate::utils::zeroed_box;

pub type ReadHandler = fn(&mut Cart, addr: u32) -> u8;
pub type WriteHandler = fn(&mut Cart, addr: u32, value: u8);

/// Mirror `addr` back to the `0..size` range, by removing progressively smaller power-of-two sizes
/// from it (emulating mirroring behavior on real hardware; i.e., a 96 MiB region would be mirrored
/// as 64 MiB, then 32 MiB, then a mirror of those 32 MiB, then mirrors of the entire 128 MiB
/// region). For example, `mirror(0x1A1A, 0x1800)` will result in `0x121A`.
pub(super) fn mirror(mut addr: u32, mut size: u32) -> u32 {
    let mut base = 0;
    let mut mask = 1 << 23;
    while addr >= size {
        while addr & mask == 0 {
            mask >>= 1;
        }
        addr -= mask;
        if size > mask {
            size -= mask;
            base += mask;
        }
        mask >>= 1;
    }
    base + addr
}

/// Remove all bits present in `mask` from `addr` starting at the bottom, moving the remaining top
/// bits down.
/// For example, for `reduce(0x1234, 0x900)`, the returned value is `0x534`:
/// ```text
/// mask:  0x900 == 0b0000_1001_0000_0000 -> 0b0000_0100_0000_0000 -> 0
///                           V                      V
/// addr: 0x1234 == 0b0001_0010_0011_0100 -> 0b0000_1001_0011_0100 -> 0b0000_0101_0011_0100 == 0x534
/// ```
/// This function can be useful to calculate the offset of an address in a memory region that is
/// discontinuously mapped, but with increasing addresses, by setting `mask` to the negation of
/// the masks encompassing all mapped regions; for example, a region mapped at addresses
/// `0x0000`-`0x3FFF` and `0x8000`-`0xBFFF` of every SNES bank (`0x00`-`0xFF` in bits 16-23 of the
/// address) would result in the mask `!(0xBFFF | 0xFF0000)` = `0x4000`. Address `0x1234` would
/// end up being at an offset of `0x1234` in the backing memory, while address `0x9234`, having
/// skipped the region at `0x4000`-`0x7FFF`, would be at `0x5234`.
pub(super) fn reduce(mut addr: u32, mut mask: u32) -> u32 {
    while mask != 0 {
        let bits = (mask & -(mask as i32) as u32) - 1;
        addr = (addr >> 1 & !bits) | (addr & bits);
        mask = (mask & (mask - 1)) >> 1;
    }
    addr
}

pub type Index = u16;

#[derive(Clone)]
pub struct Map {
    pub read_offsets: Box<[Index; Self::ENTRIES]>,
    pub write_offsets: Box<[Index; Self::ENTRIES]>,
    pub read_fns: Box<[Option<ReadHandler>; Self::ENTRIES]>,
    pub write_fns: Box<[Option<WriteHandler>; Self::ENTRIES]>,
}

impl Map {
    pub const PAGE_SIZE_SHIFT: usize = 9;
    pub const PAGE_SIZE: usize = 1 << Self::PAGE_SIZE_SHIFT;
    pub const ENTRIES: usize = 1 << (24 - Self::PAGE_SIZE_SHIFT);

    pub fn new() -> Self {
        Map {
            read_offsets: zeroed_box(),
            write_offsets: zeroed_box(),
            read_fns: Box::new([None; Self::ENTRIES]),
            write_fns: Box::new([None; Self::ENTRIES]),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn map<const READ: bool, const WRITE: bool>(
        &mut self,
        read_fn: Option<ReadHandler>,
        write_fn: Option<WriteHandler>,
        bank_range: (u8, u8),
        addr_range: (u16, u16),
        offset: u32,
        size: u32,
        mask: u32,
    ) {
        assert!(
            addr_range.0 & (Self::PAGE_SIZE - 1) as u16 == 0,
            "map address range start must be aligned to the page size"
        );
        assert!(
            addr_range.1 & (Self::PAGE_SIZE - 1) as u16 == (Self::PAGE_SIZE - 1) as u16,
            "map address range end must be aligned to the page size"
        );
        for bank_base in
            (((bank_range.0 as u32) << 16)..=((bank_range.1 as u32) << 16)).step_by(1 << 16)
        {
            for addr in ((bank_base | addr_range.0 as u32)..=(bank_base | addr_range.1 as u32))
                .step_by(Self::PAGE_SIZE)
            {
                let i = (addr >> Self::PAGE_SIZE_SHIFT) as usize;
                let page_offset = offset + mirror(reduce(addr, mask), size);
                if READ {
                    self.read_fns[i] = read_fn;
                    self.read_offsets[i] = (page_offset >> Self::PAGE_SIZE_SHIFT) as Index;
                }
                if WRITE {
                    self.write_fns[i] = write_fn;
                    self.write_offsets[i] = (page_offset >> Self::PAGE_SIZE_SHIFT) as Index;
                }
            }
        }
    }

    #[inline]
    pub fn read_data(&self, addr: u32) -> Option<(ReadHandler, u32)> {
        let i = addr as usize >> Self::PAGE_SIZE_SHIFT & (Self::ENTRIES - 1);
        self.read_fns[i].map(|entry| {
            (
                entry,
                (self.read_offsets[i] as u32) << Self::PAGE_SIZE_SHIFT
                    | (addr & (Self::PAGE_SIZE - 1) as u32),
            )
        })
    }

    #[inline]
    pub fn write_data(&self, addr: u32) -> Option<(WriteHandler, u32)> {
        let i = addr as usize >> Self::PAGE_SIZE_SHIFT & (Self::ENTRIES - 1);
        self.write_fns[i].map(|entry| {
            (
                entry,
                (self.write_offsets[i] as u32) << Self::PAGE_SIZE_SHIFT
                    | (addr & (Self::PAGE_SIZE - 1) as u32),
            )
        })
    }
}
