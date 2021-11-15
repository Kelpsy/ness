use super::FreqCounter;
use crate::{
    apu::Apu,
    utils::{bitfield_debug, bounded_int_lit},
};

#[rustfmt::skip]
static GAUSS_TABLES: [[i16; 0x100]; 2] = [
    [
        0x000, 0x000, 0x000, 0x000, 0x000, 0x000, 0x000, 0x000, 0x000, 0x000, 0x000, 0x000, 0x000, 0x000, 0x000, 0x000, 
        0x001, 0x001, 0x001, 0x001, 0x001, 0x001, 0x001, 0x001, 0x001, 0x001, 0x001, 0x002, 0x002, 0x002, 0x002, 0x002, 
        0x002, 0x002, 0x003, 0x003, 0x003, 0x003, 0x003, 0x004, 0x004, 0x004, 0x004, 0x004, 0x005, 0x005, 0x005, 0x005, 
        0x006, 0x006, 0x006, 0x006, 0x007, 0x007, 0x007, 0x008, 0x008, 0x008, 0x009, 0x009, 0x009, 0x00A, 0x00A, 0x00A, 
        0x00B, 0x00B, 0x00B, 0x00C, 0x00C, 0x00D, 0x00D, 0x00E, 0x00E, 0x00F, 0x00F, 0x00F, 0x010, 0x010, 0x011, 0x011, 
        0x012, 0x013, 0x013, 0x014, 0x014, 0x015, 0x015, 0x016, 0x017, 0x017, 0x018, 0x018, 0x019, 0x01A, 0x01B, 0x01B, 
        0x01C, 0x01D, 0x01D, 0x01E, 0x01F, 0x020, 0x020, 0x021, 0x022, 0x023, 0x024, 0x024, 0x025, 0x026, 0x027, 0x028, 
        0x029, 0x02A, 0x02B, 0x02C, 0x02D, 0x02E, 0x02F, 0x030, 0x031, 0x032, 0x033, 0x034, 0x035, 0x036, 0x037, 0x038, 
        0x03A, 0x03B, 0x03C, 0x03D, 0x03E, 0x040, 0x041, 0x042, 0x043, 0x045, 0x046, 0x047, 0x049, 0x04A, 0x04C, 0x04D, 
        0x04E, 0x050, 0x051, 0x053, 0x054, 0x056, 0x057, 0x059, 0x05A, 0x05C, 0x05E, 0x05F, 0x061, 0x063, 0x064, 0x066, 
        0x068, 0x06A, 0x06B, 0x06D, 0x06F, 0x071, 0x073, 0x075, 0x076, 0x078, 0x07A, 0x07C, 0x07E, 0x080, 0x082, 0x084, 
        0x086, 0x089, 0x08B, 0x08D, 0x08F, 0x091, 0x093, 0x096, 0x098, 0x09A, 0x09C, 0x09F, 0x0A1, 0x0A3, 0x0A6, 0x0A8, 
        0x0AB, 0x0AD, 0x0AF, 0x0B2, 0x0B4, 0x0B7, 0x0BA, 0x0BC, 0x0BF, 0x0C1, 0x0C4, 0x0C7, 0x0C9, 0x0CC, 0x0CF, 0x0D2, 
        0x0D4, 0x0D7, 0x0DA, 0x0DD, 0x0E0, 0x0E3, 0x0E6, 0x0E9, 0x0EC, 0x0EF, 0x0F2, 0x0F5, 0x0F8, 0x0FB, 0x0FE, 0x101, 
        0x104, 0x107, 0x10B, 0x10E, 0x111, 0x114, 0x118, 0x11B, 0x11E, 0x122, 0x125, 0x129, 0x12C, 0x130, 0x133, 0x137, 
        0x13A, 0x13E, 0x141, 0x145, 0x148, 0x14C, 0x150, 0x153, 0x157, 0x15B, 0x15F, 0x162, 0x166, 0x16A, 0x16E, 0x172, 
    ],
    [
        0x176, 0x17A, 0x17D, 0x181, 0x185, 0x189, 0x18D, 0x191, 0x195, 0x19A, 0x19E, 0x1A2, 0x1A6, 0x1AA, 0x1AE, 0x1B2, 
        0x1B7, 0x1BB, 0x1BF, 0x1C3, 0x1C8, 0x1CC, 0x1D0, 0x1D5, 0x1D9, 0x1DD, 0x1E2, 0x1E6, 0x1EB, 0x1EF, 0x1F3, 0x1F8, 
        0x1FC, 0x201, 0x205, 0x20A, 0x20F, 0x213, 0x218, 0x21C, 0x221, 0x226, 0x22A, 0x22F, 0x233, 0x238, 0x23D, 0x241, 
        0x246, 0x24B, 0x250, 0x254, 0x259, 0x25E, 0x263, 0x267, 0x26C, 0x271, 0x276, 0x27B, 0x280, 0x284, 0x289, 0x28E, 
        0x293, 0x298, 0x29D, 0x2A2, 0x2A6, 0x2AB, 0x2B0, 0x2B5, 0x2BA, 0x2BF, 0x2C4, 0x2C9, 0x2CE, 0x2D3, 0x2D8, 0x2DC, 
        0x2E1, 0x2E6, 0x2EB, 0x2F0, 0x2F5, 0x2FA, 0x2FF, 0x304, 0x309, 0x30E, 0x313, 0x318, 0x31D, 0x322, 0x326, 0x32B, 
        0x330, 0x335, 0x33A, 0x33F, 0x344, 0x349, 0x34E, 0x353, 0x357, 0x35C, 0x361, 0x366, 0x36B, 0x370, 0x374, 0x379, 
        0x37E, 0x383, 0x388, 0x38C, 0x391, 0x396, 0x39B, 0x39F, 0x3A4, 0x3A9, 0x3AD, 0x3B2, 0x3B7, 0x3BB, 0x3C0, 0x3C5, 
        0x3C9, 0x3CE, 0x3D2, 0x3D7, 0x3DC, 0x3E0, 0x3E5, 0x3E9, 0x3ED, 0x3F2, 0x3F6, 0x3FB, 0x3FF, 0x403, 0x408, 0x40C, 
        0x410, 0x415, 0x419, 0x41D, 0x421, 0x425, 0x42A, 0x42E, 0x432, 0x436, 0x43A, 0x43E, 0x442, 0x446, 0x44A, 0x44E, 
        0x452, 0x455, 0x459, 0x45D, 0x461, 0x465, 0x468, 0x46C, 0x470, 0x473, 0x477, 0x47A, 0x47E, 0x481, 0x485, 0x488, 
        0x48C, 0x48F, 0x492, 0x496, 0x499, 0x49C, 0x49F, 0x4A2, 0x4A6, 0x4A9, 0x4AC, 0x4AF, 0x4B2, 0x4B5, 0x4B7, 0x4BA, 
        0x4BD, 0x4C0, 0x4C3, 0x4C5, 0x4C8, 0x4CB, 0x4CD, 0x4D0, 0x4D2, 0x4D5, 0x4D7, 0x4D9, 0x4DC, 0x4DE, 0x4E0, 0x4E3, 
        0x4E5, 0x4E7, 0x4E9, 0x4EB, 0x4ED, 0x4EF, 0x4F1, 0x4F3, 0x4F5, 0x4F6, 0x4F8, 0x4FA, 0x4FB, 0x4FD, 0x4FF, 0x500, 
        0x502, 0x503, 0x504, 0x506, 0x507, 0x508, 0x50A, 0x50B, 0x50C, 0x50D, 0x50E, 0x50F, 0x510, 0x511, 0x511, 0x512, 
        0x513, 0x514, 0x514, 0x515, 0x516, 0x516, 0x517, 0x517, 0x517, 0x518, 0x518, 0x518, 0x518, 0x518, 0x519, 0x519, 
    ]
];

bounded_int_lit!(pub struct Index(u8), max 7);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Stopped,
    JustStarted(u8),
    Adsr,
    DirectGain,
    CustomGain,
    Release,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Attack,
    Decay,
    Sustain,
}

bitfield_debug! {
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct AdsrControl(pub u16) {
        pub attack_rate: u8 @ 0..=3,
        pub decay_rate: u8 @ 4..=6,
        pub use_adsr: bool @ 7,
        pub sustain_rate: u8 @ 8..=12,
        pub sustain_level: u8 @ 13..=15,
    }
}

bitfield_debug! {
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct GainControl(pub u8) {
        pub fixed_volume: u8 @ 0..=6,
        pub gain_rate: u8 @ 0..=4,
        pub gain_mode: u8 @ 5..=6,
        pub use_custom_gain: bool @ 7,
    }
}

bounded_int_lit!(pub struct Filter(u8), max 3);

bitfield_debug! {
    #[derive(Clone, Copy, PartialEq, Eq)]
    struct BrrHeader(pub u8) {
        pub loop_end_flag: u8 @ 0..=1,
        pub filter: u8 @ 2..=3,
        pub shift_amount: u8 @ 4..=7,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BrrBlockEnd {
    Normal,
    Mute,
    Loop,
}

#[derive(Clone, Copy, Debug)]
pub struct Channel {
    pub volume: [i8; 2],
    pub pitch: u16,
    pub source_number: u8,
    adsr_control: AdsrControl,
    pub gain_control: GainControl,
    pub envelope: u8,
    pub last_sample: i8,

    cur_addr: u16,
    loop_addr: u16,

    pub(super) pitch_counter: u16,

    last_brr_samples: [i16; 4],
    brr_samples: [(i16, Filter); 20],
    brr_block_end: BrrBlockEnd,
    last_sample_index: u8,

    state: State,
    mode: Mode,
    internal_envelope: u16,
    envelope_counter: FreqCounter,
    envelope_step: u16,
    envelope_sustain_level: u16,
    direct_gain_envelope: u16,
}

impl Channel {
    pub(super) fn new() -> Self {
        Channel {
            volume: [0; 2],
            pitch: 0,
            source_number: 0,
            adsr_control: AdsrControl(0),
            gain_control: GainControl(0),
            envelope: 0,
            last_sample: 0,

            cur_addr: 0,
            loop_addr: 0,

            pitch_counter: 0,

            last_brr_samples: [0; 4],
            brr_samples: [(0, Filter::new(0)); 20],
            brr_block_end: BrrBlockEnd::Normal,
            last_sample_index: 0,

            state: State::Stopped,
            mode: Mode::Attack,
            internal_envelope: 0,
            envelope_counter: FreqCounter::new(),
            envelope_step: 0,
            envelope_sustain_level: 0x100,
            direct_gain_envelope: 0,
        }
    }

    pub(super) fn check_stopped(apu: &mut Apu, i: Index) -> bool {
        let channel = &mut apu.dsp.channels[i.get() as usize];
        match &mut channel.state {
            State::Stopped => true,
            State::JustStarted(remaining) => {
                if *remaining == 0 {
                    if channel.adsr_control.use_adsr() {
                        channel.recalc_adsr_envelope_values(true);
                    } else {
                        channel.recalc_gain_envelope_values(true);
                    }
                    Channel::read_next_brr_block(apu, i);
                    false
                } else {
                    *remaining -= 1;
                    true
                }
            }
            _ => false,
        }
    }

    #[inline]
    pub fn adsr_control(&self) -> AdsrControl {
        self.adsr_control
    }

    fn recalc_adsr_envelope_values(&mut self, reset: bool) {
        if reset {
            self.state = State::Adsr;
        }
        match self.mode {
            Mode::Attack => {
                self.envelope_counter
                    .set_rate(self.adsr_control.attack_rate() << 1 | 1, reset);
                self.envelope_step = if self.adsr_control.attack_rate() == 0xF {
                    1024
                } else {
                    32
                };
            }
            Mode::Decay => {
                self.envelope_counter
                    .set_rate((self.adsr_control.decay_rate() << 1) + 16, false);
            }
            Mode::Sustain => self
                .envelope_counter
                .set_rate(self.adsr_control.sustain_rate(), false),
        }
    }

    fn recalc_gain_envelope_values(&mut self, reset: bool) {
        if reset {
            self.state = if self.gain_control.use_custom_gain() {
                State::CustomGain
            } else {
                State::DirectGain
            };
        }
        match self.state {
            State::DirectGain => {
                self.direct_gain_envelope = (self.gain_control.fixed_volume() as u16) << 4;
            }
            State::CustomGain => self
                .envelope_counter
                .set_rate(self.gain_control.gain_rate(), reset),
            _ => {}
        }
    }

    fn enter_release_state(&mut self) {
        self.state = State::Release;
        self.envelope_counter.reset();
    }

    pub fn set_adsr_control(&mut self, value: AdsrControl) {
        let prev = self.adsr_control;
        self.adsr_control = value;
        self.envelope_sustain_level = (value.sustain_level() as u16 + 1) << 8;
        if self.adsr_control.use_adsr() {
            self.recalc_adsr_envelope_values(!prev.use_adsr());
        } else if !prev.use_adsr() {
            self.recalc_gain_envelope_values(prev.use_adsr());
        }
    }

    pub fn gain_control(&self) -> GainControl {
        self.gain_control
    }

    pub fn set_gain_control(&mut self, value: GainControl) {
        let prev = self.gain_control;
        self.gain_control = value;
        if !self.adsr_control.use_adsr() {
            self.recalc_gain_envelope_values(
                self.gain_control.use_custom_gain() != prev.use_custom_gain(),
            );
        }
    }

    pub(super) fn set_enabled<const ENABLED: bool>(apu: &mut Apu, i: Index) {
        let channel = &mut apu.dsp.channels[i.get() as usize];
        if ENABLED {
            let entry_addr = ((apu.dsp.sample_table_base as u16) << 8)
                .wrapping_add((channel.source_number as u16) << 2);
            channel.cur_addr = apu.spc700.memory.read_le(entry_addr as usize);
            channel.loop_addr = apu.spc700.memory.read_le(entry_addr as usize | 2);
            channel.pitch_counter = 0;
            channel.last_brr_samples.fill(0);
            channel.brr_samples.fill((0, Filter::new(0)));
            channel.brr_block_end = BrrBlockEnd::Normal;

            channel.state = State::JustStarted(5);
            channel.mode = Mode::Attack;
            channel.internal_envelope = 0;
            channel.last_sample_index = 19;

            apu.dsp.ended_channels &= !(1 << i.get());
        } else if matches!(channel.state, State::JustStarted(_) | State::Stopped) {
            channel.state = State::Stopped;
        } else {
            channel.enter_release_state();
        }
    }

    pub(super) fn read_next_brr_block(apu: &mut Apu, i: Index) {
        let channel = &mut apu.dsp.channels[i.get() as usize];
        channel.last_sample_index -= 16;
        if channel.brr_block_end != BrrBlockEnd::Normal {
            channel.cur_addr = channel.loop_addr;
            apu.dsp.ended_channels |= 1 << i.get();
            if channel.brr_block_end == BrrBlockEnd::Mute {
                channel.enter_release_state();
                channel.internal_envelope = 0;
            }
        }
        channel.brr_samples.copy_within(16.., 0);
        let header = BrrHeader(apu.spc700.memory[channel.cur_addr as usize]);
        channel.cur_addr = channel.cur_addr.wrapping_add(1);
        let shift_amount = header.shift_amount();
        let filter = Filter::new(header.filter());
        channel.brr_block_end = match header.loop_end_flag() {
            0 | 2 => BrrBlockEnd::Normal,
            1 => BrrBlockEnd::Mute,
            _ => BrrBlockEnd::Loop,
        };
        let brr_samples = &mut channel.brr_samples[4..];
        for i in 0..8 {
            let byte = apu.spc700.memory[channel.cur_addr as usize] as i8 as i16;
            channel.cur_addr = channel.cur_addr.wrapping_add(1);
            for (i, sample) in [(i << 1, byte >> 4), (i << 1 | 1, byte << 12 >> 12)] {
                brr_samples[i] = (
                    if shift_amount > 12 {
                        sample >> 3 << 11
                    } else {
                        sample << shift_amount >> 1
                    },
                    filter,
                );
            }
        }
    }

    pub(super) fn update_stopped(apu: &mut Apu, i: Index) {
        let channel = &mut apu.dsp.channels[i.get() as usize];
        channel.envelope = (channel.internal_envelope >> 4) as u8;
        channel.last_sample = 0;
    }

    pub(super) fn output_sample(apu: &mut Apu, i: Index) -> i16 {
        let channel = &mut apu.dsp.channels[i.get() as usize];
        let sample = {
            let sample_index = 4 + (channel.pitch_counter >> 12) as u8;
            for i in channel.last_sample_index..sample_index {
                let (sample, filter) = channel.brr_samples[i as usize];
                let sample = sample as i32;
                let filtered_sample = match filter.get() {
                    0 => sample,
                    1 => {
                        let old = channel.last_brr_samples[3] as i32;
                        sample + old - (old >> 4)
                    }
                    2 => {
                        let old = channel.last_brr_samples[3] as i32;
                        let older = channel.last_brr_samples[2] as i32;
                        sample + (old << 1) - ((old * 3) >> 5) - older + (older >> 4)
                    }
                    _ => {
                        let old = channel.last_brr_samples[3] as i32;
                        let older = channel.last_brr_samples[2] as i32;
                        sample + (old << 1) - ((old * 13) >> 6) - older + ((older * 3) >> 4)
                    }
                };
                let clipped_sample = (filtered_sample.clamp(-0x8000, 0x7FFF) as i16) << 1 >> 1;
                channel.last_brr_samples.copy_within(1.., 0);
                channel.last_brr_samples[3] = clipped_sample;
            }
            channel.last_sample_index = sample_index;

            if channel.internal_envelope == 0 {
                0
            } else {
                let sample = if apu.dsp.noise_mask & 1 << i.get() == 0 {
                    let base_interp_index = ((channel.pitch_counter >> 4) & 0xFF) as usize;
                    (((GAUSS_TABLES[0][0xFF - base_interp_index] as i32
                        * channel.last_brr_samples[0] as i32)
                        >> 10) as i16)
                        .wrapping_add(
                            ((GAUSS_TABLES[1][0xFF - base_interp_index] as i32
                                * channel.last_brr_samples[1] as i32)
                                >> 10) as i16,
                        )
                        .wrapping_add(
                            ((GAUSS_TABLES[1][base_interp_index] as i32
                                * channel.last_brr_samples[2] as i32)
                                >> 10) as i16,
                        )
                        .saturating_add(
                            ((GAUSS_TABLES[0][base_interp_index] as i32
                                * channel.last_brr_samples[3] as i32)
                                >> 10) as i16,
                        )
                        >> 1
                } else {
                    apu.dsp.noise_value
                };
                ((sample as i32 * channel.internal_envelope as i32) >> 11) as i16
            }
        };

        if channel.envelope_counter.needs_update(apu.dsp_timestamp) {
            if channel.state == State::Adsr {
                match channel.mode {
                    Mode::Attack => {
                        channel.internal_envelope += channel.envelope_step;
                        if channel.internal_envelope >= 0x7E0 {
                            channel.internal_envelope = channel.internal_envelope.min(0x7FF);
                            channel.mode = Mode::Decay;
                            channel.recalc_adsr_envelope_values(false);
                        }
                    }
                    Mode::Decay => {
                        channel.internal_envelope -= ((channel.internal_envelope - 1) >> 8) + 1;
                        if channel.internal_envelope <= channel.envelope_sustain_level {
                            channel.mode = Mode::Sustain;
                            channel.recalc_adsr_envelope_values(false);
                        }
                    }
                    Mode::Sustain => {
                        channel.internal_envelope -=
                            (((channel.internal_envelope as i16 - 1) >> 8) + 1) as u16;
                    }
                }
            } else {
                match channel.state {
                    State::DirectGain => {
                        channel.internal_envelope = channel.direct_gain_envelope;
                    }
                    State::CustomGain => match channel.gain_control.gain_mode() {
                        0 => {
                            channel.internal_envelope =
                                channel.internal_envelope.saturating_sub(32);
                        }
                        1 => {
                            channel.internal_envelope -=
                                (((channel.internal_envelope as i16 - 1) >> 8) + 1) as u16;
                        }
                        2 => {
                            channel.internal_envelope = (channel.internal_envelope + 32).min(0x7FF);
                        }
                        _ => {
                            channel.internal_envelope = if channel.internal_envelope < 0x600 {
                                channel.internal_envelope + 32
                            } else {
                                (channel.internal_envelope + 8).min(0x7FF)
                            };
                        }
                    },
                    State::Release => {
                        channel.internal_envelope = channel.internal_envelope.saturating_sub(8);
                    }
                    _ => {}
                }
                match channel.mode {
                    Mode::Attack => {
                        if channel.internal_envelope >= 0x7E0 {
                            channel.mode = Mode::Decay;
                        }
                    }
                    Mode::Decay => {
                        if channel.internal_envelope <= channel.envelope_sustain_level {
                            channel.mode = Mode::Sustain;
                        }
                    }
                    Mode::Sustain => {}
                }
            }
        }

        channel.last_sample = (sample >> 7) as i8;
        channel.envelope = (channel.internal_envelope >> 4) as u8;

        sample
    }
}
