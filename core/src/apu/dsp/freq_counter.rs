use crate::schedule::Timestamp;

#[rustfmt::skip]
static STEP_RATES: [(u8, u8); 0x20] = [
    (1, (Timestamp::BITS - 1) as u8), (4, 9), (3, 9), (5, 8),
    (4, 8), (3, 8), (5, 7), (4, 7),
    (3, 7), (5, 6), (4, 6), (3, 6),
    (5, 5), (4, 5), (3, 5), (5, 4),
    (4, 4), (3, 4), (5, 3), (4, 3),
    (3, 3), (5, 2), (4, 2), (3, 2),
    (5, 1), (4, 1), (3, 1), (5, 0),
    (4, 0), (3, 0), (2, 0), (1, 0),
];

#[derive(Clone, Copy, Debug)]
pub struct FreqCounter {
    pub reset: u8,
    pub counter: u8,
    pub shift: u8,
}

impl FreqCounter {
    pub fn new() -> Self {
        FreqCounter {
            reset: 1,
            counter: 1,
            shift: 0,
        }
    }

    pub fn set_rate(&mut self, rate: u8, reset_counter: bool) {
        let (reset, shift) = STEP_RATES[rate as usize];
        self.reset = reset;
        self.shift = shift;
        if reset_counter {
            self.counter = self.reset;
        }
    }

    pub fn reset(&mut self) {
        self.reset = 1;
        self.counter = 1;
        self.shift = 0;
    }

    pub fn needs_update(&mut self, dsp_timestamp: Timestamp) -> bool {
        if (dsp_timestamp << 1 | 1 << (Timestamp::BITS - 1)) & 1 << self.shift == 0 {
            self.counter -= 1;
            if self.counter == 0 {
                self.counter = self.reset;
                return true;
            }
        }
        false
    }
}
