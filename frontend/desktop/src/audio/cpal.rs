use super::{Interp, Receiver, INPUT_SAMPLE_RATE};
use core::{
    iter,
    sync::atomic::{AtomicU32, Ordering},
};
use cpal::{
    default_host,
    platform::Stream,
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Sample, SampleFormat,
};
use std::sync::Arc;

pub struct OutputStream {
    _stream: Stream,
    interp_tx: crossbeam_channel::Sender<Box<dyn Interp>>,
    volume: Arc<AtomicU32>,
}

impl OutputStream {
    pub(super) fn new(rx: Receiver, interp: Box<dyn Interp>, volume: f32) -> Option<Self> {
        let output_device = default_host().default_output_device()?;
        let supported_output_config = output_device
            .supported_output_configs()
            .ok()?
            .find(|config| config.channels() == 2)?
            .with_max_sample_rate();

        let output_sample_rate = supported_output_config.sample_rate().0 as f64;
        let ratio = INPUT_SAMPLE_RATE / output_sample_rate;

        let (interp_tx, interp_rx) = crossbeam_channel::unbounded();
        let volume = Arc::new(AtomicU32::new(volume.to_bits()));

        let mut output_data = OutputData {
            rx,
            interp_rx,
            interp,
            volume: Arc::clone(&volume),
            ratio,
            fract: 0.0,
        };

        let err_callback = |err| panic!("Error in default audio output device stream: {}", err);
        let stream = match supported_output_config.sample_format() {
            SampleFormat::U16 => output_device.build_output_stream(
                &supported_output_config.config(),
                move |data: &mut [u16], _| output_data.fill(data),
                err_callback,
            ),
            SampleFormat::I16 => output_device.build_output_stream(
                &supported_output_config.config(),
                move |data: &mut [i16], _| output_data.fill(data),
                err_callback,
            ),
            SampleFormat::F32 => output_device.build_output_stream(
                &supported_output_config.config(),
                move |data: &mut [f32], _| output_data.fill(data),
                err_callback,
            ),
        }
        .ok()?;
        stream.play().expect("Couldn't start audio output stream");

        Some(OutputStream {
            _stream: stream,
            interp_tx,
            volume,
        })
    }

    pub fn set_interp(&mut self, interp: Box<dyn Interp>) {
        self.interp_tx
            .send(interp)
            .expect("Couldn't send new interpolator to audio thread");
    }

    pub fn volume(&mut self) -> f32 {
        f32::from_bits(self.volume.load(Ordering::Relaxed))
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume.store(volume.to_bits(), Ordering::Relaxed);
    }
}

struct OutputData {
    rx: Receiver,
    interp_rx: crossbeam_channel::Receiver<Box<dyn Interp>>,
    interp: Box<dyn Interp>,
    volume: Arc<AtomicU32>,
    ratio: f64,
    fract: f64,
}

impl OutputData {
    fn fill<T: Sample>(&mut self, data: &mut [T]) {
        if let Some(interp) = self.interp_rx.try_iter().last() {
            self.interp = interp;
        }

        let mut fract = self.fract;
        let mut output_i = 0;
        let mut volume = f32::from_bits(self.volume.load(Ordering::Relaxed));
        volume *= volume;

        let max_input_samples = (((data.len()) >> 1) as f64 * self.ratio + fract).ceil() as usize;

        macro_rules! push_output_samples {
            () => {
                while fract < 1.0 {
                    if output_i >= data.len() {
                        self.fract = fract;
                        return;
                    }
                    let result = self.interp.get_output_sample(fract);
                    data[output_i] = T::from(&(result[0] as f32 * volume));
                    data[output_i + 1] = T::from(&(result[1] as f32 * volume));
                    fract += self.ratio;
                    output_i += 2;
                }
                fract -= 1.0;
            };
        }

        for input_sample in iter::from_fn(|| self.rx.read_sample()).take(max_input_samples) {
            self.interp.push_input_sample(input_sample);
            push_output_samples!();
        }

        loop {
            self.interp.copy_last_input_sample();
            push_output_samples!();
        }
    }
}
