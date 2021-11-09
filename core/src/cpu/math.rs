// TODO: Timings

pub struct Math {
    pub multiplicand: u8,
    pub multiplier: u8,
    pub dividend: u16,
    pub divisor: u8,
    div_quotient: u16,
    mul_result_div_remainder: u16,
}

impl Math {
    pub(crate) fn new() -> Self {
        Math {
            multiplicand: 0xFF,
            multiplier: 0xFF,
            dividend: 0xFFFF,
            divisor: 0xFF,
            div_quotient: 0,
            mul_result_div_remainder: 0,
        }
    }

    #[inline]
    pub fn div_quotient(&self) -> u16 {
        self.div_quotient
    }

    #[inline]
    pub fn mul_result_div_remainder(&self) -> u16 {
        self.mul_result_div_remainder
    }

    #[inline]
    pub fn run_multiplication(&mut self) {
        self.div_quotient = self.multiplier as u16;
        self.mul_result_div_remainder = self.multiplicand as u16 * self.multiplier as u16;
    }

    pub fn run_division(&mut self) {
        if self.divisor == 0 {
            self.div_quotient = 0xFFFF;
            self.mul_result_div_remainder = self.dividend;
        } else {
            self.div_quotient = self.dividend / self.divisor as u16;
            self.mul_result_div_remainder = self.dividend % self.divisor as u16;
        }
    }
}
