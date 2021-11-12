#[derive(Clone, Copy, Debug)]
pub struct Channel {
    pub volume: [i8; 2],
    pub pitch: u16,
    pub source_number: u8,
    pub adsr_1: u8,
    pub adsr_2: u8,
    pub gain: u8,
}

pub struct Dsp {
    #[cfg(feature = "log")]
    logger: slog::Logger,
    pub channels: [Channel; 8],
    pub fir_coeffs: [i8; 8],
    pub main_volume: [i8; 2],
    pub echo_volume: [i8; 2],
    pub key_on: u8,
    pub key_off: u8,
    pub flags: u8,
    pub ended_channels: u8,
    pub echo_feedback: i8,
    pub unused: u8,
    pub pitch_mod_mask: u8,
    pub noise_mask: u8,
    pub echo_mask: u8,
    pub source_directory_off: u8,
    pub echo_buffer_off: u8,
    pub echo_delay: u8,
}

impl Dsp {
    pub(crate) fn new(#[cfg(feature = "log")] logger: slog::Logger) -> Self {
        Dsp {
            #[cfg(feature = "log")]
            logger,
            channels: [Channel {
                volume: [0; 2],
                pitch: 0,
                source_number: 0,
                adsr_1: 0,
                adsr_2: 0,
                gain: 0,
            }; 8],
            fir_coeffs: [0; 8],
            main_volume: [0; 2],
            echo_volume: [0; 2],
            key_on: 0,
            key_off: 0,
            flags: 0,
            ended_channels: 0,
            echo_feedback: 0,
            unused: 0,
            pitch_mod_mask: 0,
            noise_mask: 0,
            echo_mask: 0,
            source_directory_off: 0,
            echo_buffer_off: 0,
            echo_delay: 0,
        }
    }

    pub fn read_reg(&self, mut index: u8) -> u8 {
        index &= 0x7F;
        let i = (index >> 4) as usize;
        match index & 0xF {
            0 => self.channels[i].volume[0] as u8,
            1 => self.channels[i].volume[1] as u8,
            2 => self.channels[i].pitch as u8,
            3 => (self.channels[i].pitch >> 8) as u8,
            4 => self.channels[i].source_number,
            5 => self.channels[i].adsr_1,
            6 => self.channels[i].adsr_2,
            7 => self.channels[i].gain,
            8 => {
                #[cfg(feature = "log")]
                slog::info!(self.logger, "ENVX read @ {:#04X}", index);
                0
            }
            9 => {
                #[cfg(feature = "log")]
                slog::info!(self.logger, "OUTX read @ {:#04X}", index);
                0
            }
            0xC => match i {
                0x0 => self.main_volume[0] as u8,
                0x1 => self.main_volume[1] as u8,
                0x2 => self.echo_volume[0] as u8,
                0x3 => self.echo_volume[1] as u8,
                0x4 => self.key_on,
                0x5 => self.key_off,
                0x6 => self.flags,
                _ => self.ended_channels,
            },
            0xD => match i {
                0x0 => self.echo_feedback as u8,
                0x1 => self.unused,
                0x2 => self.pitch_mod_mask,
                0x3 => self.noise_mask,
                0x4 => self.echo_mask,
                0x5 => self.source_directory_off,
                0x6 => self.echo_buffer_off,
                _ => self.echo_delay,
            },
            0xF => self.fir_coeffs[i] as u8,
            _ => {
                #[cfg(feature = "log")]
                slog::info!(self.logger, "Read from unknown register @ {:#04X}", index);
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
            5 => self.channels[i].adsr_1 = value,
            6 => self.channels[i].adsr_2 = value,
            7 => self.channels[i].gain = value,
            0xC => match i {
                0x0 => self.main_volume[0] = value as i8,
                0x1 => self.main_volume[1] = value as i8,
                0x2 => self.echo_volume[0] = value as i8,
                0x3 => self.echo_volume[1] = value as i8,
                0x4 => self.key_on = value,
                0x5 => self.key_off = value,
                0x6 => self.flags = value,
                _ => self.ended_channels = 0,
            },
            0xD => match i {
                0x0 => self.echo_feedback = value as i8,
                0x1 => self.unused = value,
                0x2 => self.pitch_mod_mask = value,
                0x3 => self.noise_mask = value,
                0x4 => self.echo_mask = value,
                0x5 => self.source_directory_off = value,
                0x6 => self.echo_buffer_off = value,
                _ => self.echo_delay = value,
            },
            0xF => self.fir_coeffs[i] = value as i8,
            _ => {
                #[cfg(feature = "log")]
                slog::info!(
                    self.logger,
                    "Write to unknown register @ {:#04X}: {:#04X}",
                    index,
                    value,
                );
            }
        }
    }
}
