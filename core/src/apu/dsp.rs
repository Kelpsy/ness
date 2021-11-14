pub mod channel;
mod freq_counter;
mod io;
use freq_counter::FreqCounter;

use super::Apu;
use crate::utils::bitfield_debug;
use channel::{Channel, Index};

pub type Sample = i16;

pub trait Backend {
    fn handle_sample_chunk(&mut self, samples: &[[Sample; 2]]);
}

pub struct DummyBackend;

impl Backend for DummyBackend {
    fn handle_sample_chunk(&mut self, _samples: &[[Sample; 2]]) {}
}

bitfield_debug! {
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct Flags(pub u8) {
        pub noise_freq: u8 @ 0..=4,
        pub write_to_echo_buffer: bool @ 5,
        pub mute_amplifier: bool @ 6,
        pub soft_reset: bool @ 7,
    }
}

pub struct Dsp {
    #[cfg(feature = "log")]
    logger: slog::Logger,
    pub backend: Box<dyn Backend>,
    sample_chunk_len: usize,
    sample_chunk: Vec<[Sample; 2]>,

    pub channels: [Channel; 8],
    pub main_volume: [i8; 2],
    pub flags: Flags,
    pub unused: u8,
    pub pitch_mod_mask: u8,
    pub sample_table_base: u8,

    pub key_on: u8,
    pub key_off: u8,
    internal_key_on: u8,
    internal_key_off: u8,

    ended_channels: u8,

    pub noise_mask: u8,
    noise_value: i16,
    noise_counter: FreqCounter,

    pub echo_volume: [i8; 2],
    pub echo_feedback_volume: i8,
    pub echo_channel_mask: u8,
    pub echo_buffer_base: u8,
    pub echo_buffer_len: u8,
    pub echo_fir_coeffs: [i8; 8],
}

impl Dsp {
    pub(crate) fn new(
        backend: Box<dyn Backend>,
        sample_chunk_len: usize,
        #[cfg(feature = "log")] logger: slog::Logger,
    ) -> Self {
        Dsp {
            #[cfg(feature = "log")]
            logger,
            backend,
            sample_chunk_len,
            sample_chunk: Vec::new(),

            channels: [Channel::new(); 8],
            main_volume: [0; 2],
            flags: Flags(0),
            unused: 0,
            pitch_mod_mask: 0,
            sample_table_base: 0,

            key_on: 0,
            key_off: 0,
            internal_key_on: 0,
            internal_key_off: 0,

            ended_channels: 0,

            noise_mask: 0,
            noise_value: -0x4000,
            noise_counter: FreqCounter::new(),

            echo_volume: [0; 2],
            echo_feedback_volume: 0,
            echo_channel_mask: 0,
            echo_buffer_base: 0,
            echo_buffer_len: 0,
            echo_fir_coeffs: [0; 8],
        }
    }

    pub(super) fn output_sample(apu: &mut Apu) {
        if apu.dsp_timestamp & 1 == 0 && apu.dsp.internal_key_on | apu.dsp.internal_key_off != 0 {
            for i in 0..8 {
                if apu.dsp.internal_key_off & 1 << i != 0 {
                    Channel::set_enabled::<false>(apu, Index::new(i));
                } else if apu.dsp.internal_key_on & 1 << i != 0 {
                    Channel::set_enabled::<true>(apu, Index::new(i));
                }
            }
            apu.dsp.internal_key_on = 0;
            apu.dsp.internal_key_off = 0;
        }

        if apu.dsp.noise_counter.needs_update(apu.dsp_timestamp) {
            let prev = apu.dsp.noise_value as u16;
            apu.dsp.noise_value = ((prev & 0x7FFE) | ((prev ^ prev >> 1) & 1) << 15) as i16 >> 1;
        }

        let mut prev_output = 0_i16;
        let mut left_output = 0_i16;
        let mut right_output = 0_i16;

        for i in 0..8 {
            let i_ = Index::new(i as u8);
            let stopped = Channel::check_stopped(apu, i_);
            if stopped {
                Channel::update_stopped(apu, i_);
            } else {
                let output = Channel::output_sample(apu, i_);
                let channel = &mut apu.dsp.channels[i];
                left_output = left_output
                    .saturating_add(((output as i32 * channel.volume[0] as i32) >> 6) as i16);
                right_output = right_output
                    .saturating_add(((output as i32 * channel.volume[1] as i32) >> 6) as i16);
                let mut step = channel.pitch & 0x3FFF;
                if (apu.dsp.pitch_mod_mask & !1) & 1 << i != 0 {
                    step = ((step as u32 * ((prev_output >> 4) + 0x400) as u32) >> 10).min(0x3FFF)
                        as u16;
                }
                let (new_counter, overflowed) = channel.pitch_counter.overflowing_add(step);
                channel.pitch_counter = new_counter;
                prev_output = output;
                if overflowed {
                    Channel::read_next_brr_block(apu, i_);
                }
            }
        }

        if apu.dsp.flags.mute_amplifier() {
            left_output = -1;
            right_output = -1;
        } else {
            left_output = !(((left_output as i32 * apu.dsp.main_volume[0] as i32) >> 7) as i16);
            right_output = !(((right_output as i32 * apu.dsp.main_volume[1] as i32) >> 7) as i16);
        }

        apu.dsp.sample_chunk.push([left_output, right_output]);
        if apu.dsp.sample_chunk.len() >= apu.dsp.sample_chunk_len {
            apu.dsp
                .backend
                .handle_sample_chunk(&apu.dsp.sample_chunk[..]);
            apu.dsp.sample_chunk.clear();
        }
    }
}
