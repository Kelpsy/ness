mod cpal;
pub use self::cpal::*;
mod interp;
pub use interp::{Interp, InterpMethod};

use core::{
    cell::UnsafeCell,
    hint::spin_loop,
    sync::atomic::{AtomicUsize, Ordering},
};
use ness_core::apu::dsp::Sample;
use std::sync::Arc;

const INPUT_SAMPLE_RATE: f64 = 32000.0;

const BUFFER_CAPACITY: usize = 0x800;
const BUFFER_MASK: usize = BUFFER_CAPACITY - 1;

#[repr(C)]
struct Buffer {
    read_pos: AtomicUsize,
    write_pos: AtomicUsize,
    data: UnsafeCell<Box<[[Sample; 2]]>>,
}

unsafe impl Sync for Buffer {}

impl Buffer {
    fn new_arc() -> Arc<Self> {
        Arc::new(Buffer {
            read_pos: AtomicUsize::new(0),
            write_pos: AtomicUsize::new(0),
            data: UnsafeCell::new(unsafe { Box::new_zeroed_slice(BUFFER_CAPACITY).assume_init() }),
        })
    }
}

#[derive(Clone)]
pub struct SenderData {
    buffer: Arc<Buffer>,
}

pub struct Sender {
    buffer: Arc<Buffer>,
    write_pos: usize,
    sync: bool,
}

impl Sender {
    pub fn new(data: &SenderData, sync: bool) -> Self {
        Sender {
            buffer: data.buffer.clone(),
            write_pos: data.buffer.write_pos.load(Ordering::Relaxed),
            sync,
        }
    }
}

impl ness_core::apu::dsp::Backend for Sender {
    fn handle_sample_chunk(&mut self, samples: &[[Sample; 2]]) {
        if self.sync {
            let max_distance = BUFFER_CAPACITY - samples.len();
            // Wait until enough samples have been played
            while self
                .write_pos
                .wrapping_sub(self.buffer.read_pos.load(Ordering::Relaxed))
                & BUFFER_MASK
                > max_distance
            {
                spin_loop();
            }
        } else {
            // Overwrite the oldest samples, attempt to move the read position to the start of the
            // oldest remaining ones
            let _ = self.buffer.read_pos.fetch_update(
                Ordering::Relaxed,
                Ordering::Relaxed,
                |read_pos| {
                    if read_pos.wrapping_sub(self.write_pos) & BUFFER_MASK < samples.len() {
                        Some((self.write_pos + samples.len() + 1) & BUFFER_MASK)
                    } else {
                        None
                    }
                },
            );
        }
        for sample in samples {
            unsafe {
                *(&mut *self.buffer.data.get()).get_unchecked_mut(self.write_pos) = *sample;
            }
            self.write_pos = (self.write_pos + 1) & BUFFER_MASK;
        }
        self.buffer
            .write_pos
            .store(self.write_pos, Ordering::Release);
    }
}

struct Receiver {
    buffer: Arc<Buffer>,
}

impl Receiver {
    fn read_sample(&mut self) -> Option<[f64; 2]> {
        if let Ok(read_pos) =
            self.buffer
                .read_pos
                .fetch_update(Ordering::AcqRel, Ordering::Acquire, |read_pos| {
                    if read_pos == self.buffer.write_pos.load(Ordering::Acquire) {
                        None
                    } else {
                        Some((read_pos + 1) & BUFFER_MASK)
                    }
                })
        {
            let result = unsafe { (&mut *self.buffer.data.get()).get_unchecked_mut(read_pos) };
            Some([
                result[0] as f64 * (1.0 / 32768.0),
                result[1] as f64 * (1.0 / 32768.0),
            ])
        } else {
            None
        }
    }
}

pub struct Channel {
    pub tx_data: SenderData,
    pub output_stream: OutputStream,
}

pub fn channel(interp_method: InterpMethod) -> Option<Channel> {
    let buffer = Buffer::new_arc();
    Some(Channel {
        tx_data: SenderData {
            buffer: Arc::clone(&buffer),
        },
        output_stream: OutputStream::new(Receiver { buffer }, interp_method.create_interp())?,
    })
}
