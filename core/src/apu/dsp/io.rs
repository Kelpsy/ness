use super::{
    channel::{AdsrControl, GainControl},
    Dsp,
};

impl Dsp {
    pub fn read_reg(&self, mut index: u8) -> u8 {
        index &= 0x7F;
        let i = (index >> 4) as usize;
        match index & 0xF {
            0 => self.channels[i].volume[0] as u8,
            1 => self.channels[i].volume[1] as u8,
            2 => self.channels[i].pitch as u8,
            3 => (self.channels[i].pitch >> 8) as u8,
            4 => self.channels[i].source_number,
            5 => self.channels[i].adsr_control().0 as u8,
            6 => (self.channels[i].adsr_control().0 >> 8) as u8,
            7 => self.channels[i].gain_control().0,
            8 => self.channels[i].envelope,
            9 => self.channels[i].last_sample as u8,
            0xC => match i {
                0x0 => self.main_volume[0] as u8,
                0x1 => self.main_volume[1] as u8,
                0x2 => self.echo_volume[0] as u8,
                0x3 => self.echo_volume[1] as u8,
                0x4 => self.key_on,
                0x5 => self.key_off,
                0x6 => self.flags.0,
                _ => self.ended_channels,
            },
            0xD => match i {
                0x0 => self.echo_feedback_volume as u8,
                0x1 => self.unused,
                0x2 => self.pitch_mod_mask,
                0x3 => self.noise_mask,
                0x4 => self.echo_channel_mask,
                0x5 => self.sample_table_base,
                0x6 => self.echo_buffer_base,
                _ => self.echo_buffer_len,
            },
            0xF => self.echo_fir_coeffs[i] as u8,
            _ => {
                #[cfg(feature = "log")]
                slog::warn!(self.logger, "Read from unknown register @ {:#04X}", index);
                0
            }
        }
    }

    pub fn write_reg(&mut self, index: u8, value: u8) {
        if index >= 0x80 {
            #[cfg(feature = "log")]
            slog::warn!(
                self.logger,
                "Write to read-only register mirror {:#04X}",
                index
            );
            return;
        }
        let i = (index >> 4 & 7) as usize;
        match index & 0xF {
            0 => self.channels[i].volume[0] = value as i8,
            1 => self.channels[i].volume[1] = value as i8,
            2 => self.channels[i].pitch = (self.channels[i].pitch & 0xFF00) | value as u16,
            3 => self.channels[i].pitch = (self.channels[i].pitch & 0xFF) | (value as u16) << 8,
            4 => self.channels[i].source_number = value,
            5 => self.channels[i].set_adsr_control(AdsrControl(
                (self.channels[i].adsr_control().0 & 0xFF00) | value as u16,
            )),
            6 => self.channels[i].set_adsr_control(AdsrControl(
                (self.channels[i].adsr_control().0 & 0xFF) | (value as u16) << 8,
            )),
            7 => self.channels[i].set_gain_control(GainControl(value)),
            8 => self.channels[i].envelope = value,
            9 => self.channels[i].last_sample = value as i8,
            0xC => match i {
                0x0 => self.main_volume[0] = value as i8,
                0x1 => self.main_volume[1] = value as i8,
                0x2 => self.echo_volume[0] = value as i8,
                0x3 => self.echo_volume[1] = value as i8,
                0x4 => {
                    self.key_on = value;
                    self.internal_key_on |= value;
                }
                0x5 => {
                    self.key_off = value;
                    self.internal_key_off |= value;
                }
                0x6 => self.flags.0 = value,
                _ => self.ended_channels = 0,
            },
            0xD => match i {
                0x0 => self.echo_feedback_volume = value as i8,
                0x1 => self.unused = value,
                0x2 => self.pitch_mod_mask = value,
                0x3 => self.noise_mask = value,
                0x4 => self.echo_channel_mask = value,
                0x5 => self.sample_table_base = value,
                0x6 => self.echo_buffer_base = value,
                _ => self.echo_buffer_len = value,
            },
            0xF => self.echo_fir_coeffs[i] = value as i8,
            _ => {
                #[cfg(feature = "log")]
                slog::warn!(
                    self.logger,
                    "Write to unknown register @ {:#04X}: {:#04X}",
                    index,
                    value,
                );
            }
        }
    }
}
