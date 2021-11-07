use core::{
    fmt::Debug,
    ops::{BitAnd, BitOr, BitXor, Not},
};

pub trait RegSize:
    Copy
    + Debug
    + Not<Output = Self>
    + BitAnd<Output = Self>
    + BitOr<Output = Self>
    + BitXor<Output = Self>
    + Ord
{
    const IS_U16: bool;
    const SIZE: usize;

    fn trunc_u16(value: u16) -> Self;
    fn zext_u8(value: u8) -> Self;
    fn as_zext_u16(self) -> u16;
    fn as_trunc_u8(self) -> u8;

    fn is_zero(self) -> bool;
    fn is_negative(self) -> bool;

    fn update_u16_low(self, value: &mut u16);
    fn wrapping_add(self, other: Self) -> Self;
    fn wrapping_sub(self, other: Self) -> Self;
}

impl RegSize for u8 {
    const IS_U16: bool = false;
    const SIZE: usize = 1;

    fn trunc_u16(value: u16) -> Self {
        value as u8
    }
    fn zext_u8(value: u8) -> Self {
        value
    }
    fn as_zext_u16(self) -> u16 {
        self as u16
    }
    fn as_trunc_u8(self) -> u8 {
        self
    }

    fn is_zero(self) -> bool {
        self == 0
    }
    fn is_negative(self) -> bool {
        self >> 7 != 0
    }

    fn update_u16_low(self, value: &mut u16) {
        *value = (*value & 0xFF00) | self as u16;
    }
    fn wrapping_add(self, other: Self) -> Self {
        u8::wrapping_add(self, other)
    }
    fn wrapping_sub(self, other: Self) -> Self {
        u8::wrapping_sub(self, other)
    }
}

impl RegSize for u16 {
    const IS_U16: bool = true;
    const SIZE: usize = 2;

    fn trunc_u16(value: u16) -> Self {
        value
    }
    fn zext_u8(value: u8) -> Self {
        value as u16
    }
    fn as_zext_u16(self) -> u16 {
        self
    }
    fn as_trunc_u8(self) -> u8 {
        self as u8
    }

    fn is_zero(self) -> bool {
        self == 0
    }
    fn is_negative(self) -> bool {
        self >> 15 != 0
    }

    fn update_u16_low(self, value: &mut u16) {
        *value = self;
    }
    fn wrapping_add(self, other: Self) -> Self {
        u16::wrapping_add(self, other)
    }
    fn wrapping_sub(self, other: Self) -> Self {
        u16::wrapping_sub(self, other)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AddrMode {
    Immediate,           // #nn
    Direct,              // [#nn]
    DirectX,             // [#nn + X]
    DirectY,             // [#nn + Y]
    DirectIndirect,      // [(#nn)]
    DirectXIndirect,     // [(#nn + X)]
    DirectIndirectY,     // [(#nn) + Y]
    DirectIndirectLong,  // [[#nn]]
    DirectIndirectLongY, // [[#nn] + Y]
    Absolute,            // [#nnnn]
    AbsoluteX,           // [#nnnn + X]
    AbsoluteY,           // [#nnnn + Y]
    AbsoluteLong,        // [#nnnnnn]
    AbsoluteLongX,       // [#nnnnnn + X]
    StackRel,            // [#nn + S]
    StackRelIndirectY,   // [(#nn + S) + Y]
}

impl AddrMode {
    pub const fn is_masked_to_direct_page(self) -> bool {
        matches!(
            self,
            Self::Direct | Self::DirectX | Self::DirectY | Self::StackRel
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JumpAddr {
    Absolute,             // #nnnn
    AbsoluteLong,         // #nnnnnn
    AbsoluteIndirect,     // (#nnnn)
    AbsoluteIndirectLong, // [#nnnn]
    AbsoluteXIndirect,    // (#nnnn + X)
}
