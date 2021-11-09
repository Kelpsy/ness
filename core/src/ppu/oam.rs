use super::Ppu;
use crate::{cpu::bus::AccessType, utils::bitfield_debug};

bitfield_debug! {
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct Attrs(pub u8) {
        pub tile_table: bool @ 0,
        pub large_size: bool @ 1,
        pub x_flip: bool @ 6,
        pub y_flip: bool @ 7,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Obj {
    pub x_coord: u16,
    pub y_coord: u8,
    pub tile_number: u8,
    pub pal_number: u8,
    pub bg_prio: u8,
    pub attrs: Attrs,
}

pub struct Oam {
    pub contents: Box<[Obj; 0x80]>,
    cur_byte_addr: u16,
    reload_addr: u16,
    write_latch: u8,
    start_prio_at_cur_sprite: bool,
    next_first_sprite: u8,
}

impl Oam {
    pub(crate) fn new() -> Self {
        Oam {
            contents: Box::new(
                [Obj {
                    x_coord: 0,
                    y_coord: 0,
                    tile_number: 0,
                    pal_number: 0,
                    bg_prio: 0,
                    attrs: Attrs(0),
                }; 0x80],
            ),
            cur_byte_addr: 0,
            reload_addr: 0,
            write_latch: 0,
            start_prio_at_cur_sprite: false,
            next_first_sprite: 0,
        }
    }

    pub(super) fn reload_cur_byte_addr(&mut self) {
        self.cur_byte_addr = (self.reload_addr as u16) << 1;
        if self.start_prio_at_cur_sprite {
            self.next_first_sprite = (self.cur_byte_addr >> 2) as u8 & 0x7F;
        } else {
            self.next_first_sprite = 0;
        }
    }

    #[inline]
    pub fn cur_byte_addr(&self) -> u16 {
        self.cur_byte_addr
    }

    #[inline]
    pub fn reload_addr(&self) -> u16 {
        self.reload_addr
    }

    #[inline]
    pub fn set_reload_addr_low(&mut self, value: u8) {
        self.reload_addr = (self.reload_addr & 0x100) | value as u16;
        self.reload_cur_byte_addr();
    }

    #[inline]
    pub fn set_reload_addr_high(&mut self, value: u8) {
        self.reload_addr = (self.reload_addr & 0xFF) | ((value as u16) << 8 & 0x100);
        self.start_prio_at_cur_sprite = value & 0x80 != 0;
        self.reload_cur_byte_addr();
    }

    #[inline]
    pub fn write_latch(&self) -> u8 {
        self.write_latch
    }

    #[inline]
    pub fn start_prio_at_cur_sprite(&self) -> bool {
        self.start_prio_at_cur_sprite
    }

    #[inline]
    pub fn next_first_sprite(&self) -> u8 {
        self.next_first_sprite
    }
}

impl Ppu {
    fn update_oam_next_first_sprite(&mut self) {
        if self.oam.start_prio_at_cur_sprite {
            self.oam.next_first_sprite = if self.oam.cur_byte_addr & 3 == 3 {
                (self.oam.cur_byte_addr >> 2) + self.counters.v_counter()
            } else {
                self.oam.cur_byte_addr >> 2
            } as u8
                & 0x7F;
        }
    }

    pub fn read_oam<A: AccessType>(&mut self) -> u8 {
        let result = if self.oam.cur_byte_addr & 0x200 != 0 {
            let i = (self.oam.cur_byte_addr as usize & 0x1F) << 2;
            let objs = (
                &self.oam.contents[i],
                &self.oam.contents[i | 1],
                &self.oam.contents[i | 2],
                &self.oam.contents[i | 3],
            );
            (objs.0.x_coord >> 8) as u8
                | (objs.0.attrs.large_size() as u8) << 1
                | ((objs.1.x_coord >> 8) as u8) << 2
                | (objs.1.attrs.large_size() as u8) << 3
                | ((objs.2.x_coord >> 8) as u8) << 4
                | (objs.2.attrs.large_size() as u8) << 5
                | ((objs.3.x_coord >> 8) as u8) << 6
                | (objs.3.attrs.large_size() as u8) << 7
        } else {
            let i = (self.oam.cur_byte_addr >> 2) as usize & 0x7F;
            let obj = &self.oam.contents[i];
            match self.oam.cur_byte_addr & 3 {
                0 => obj.x_coord as u8,
                1 => obj.y_coord,
                2 => obj.tile_number,
                _ => (obj.attrs.0 & 0xC1) | obj.bg_prio << 4 | obj.pal_number << 1,
            }
        };
        if A::SIDE_EFFECTS {
            self.oam.cur_byte_addr = (self.oam.cur_byte_addr + 1) & 0x3FF;
            self.update_oam_next_first_sprite();
            self.ppu1_mdr = result;
        }
        result
    }

    pub fn write_oam(&mut self, value: u8) {
        if self.oam.cur_byte_addr & 1 == 0 {
            self.oam.write_latch = value;
        }
        if self.oam.cur_byte_addr & 0x200 != 0 {
            let i = (self.oam.cur_byte_addr as usize & 0x1F) << 2;
            let objs = &mut self.oam.contents[i..=i | 3];
            objs[0].x_coord = (objs[0].x_coord & 0xFF) | (value as u16 & 1) << 8;
            objs[0].attrs.set_large_size(value & 1 << 1 != 0);
            objs[1].x_coord = (objs[1].x_coord & 0xFF) | (value as u16 >> 2 & 1) << 8;
            objs[1].attrs.set_large_size(value & 1 << 3 != 0);
            objs[2].x_coord = (objs[2].x_coord & 0xFF) | (value as u16 >> 4 & 1) << 8;
            objs[2].attrs.set_large_size(value & 1 << 5 != 0);
            objs[3].x_coord = (objs[3].x_coord & 0xFF) | (value as u16 >> 6 & 1) << 8;
            objs[3].attrs.set_large_size(value & 1 << 7 != 0);
        } else if self.oam.cur_byte_addr & 1 != 0 {
            let i = (self.oam.cur_byte_addr >> 2) as usize & 0x7F;
            let obj = &mut self.oam.contents[i];
            if self.oam.cur_byte_addr & 2 == 0 {
                obj.x_coord = (obj.x_coord & !0xFF) | self.oam.write_latch as u16;
                obj.y_coord = value;
            } else {
                obj.tile_number = self.oam.write_latch;
                obj.attrs.0 = (obj.attrs.0 & !0xC1) | (value & 0xC1);
                obj.bg_prio = value >> 4 & 3;
                obj.pal_number = value >> 1 & 7;
            }
        }
        self.oam.cur_byte_addr = (self.oam.cur_byte_addr + 1) & 0x3FF;
        self.update_oam_next_first_sprite();
    }
}
