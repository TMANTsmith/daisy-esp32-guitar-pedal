use libm::roundf; // for f32
use alloc::boxed::Box;

const SAMPLE_RATE: u32 = 48000;

fn ms_to_samples(ms: usize) -> usize {
    48000 * ms / 1000
}


pub struct Delay<const MS: usize> {
    decay_factor: f32,          // feedback/mix
    buffer: Box<[(f32, f32); ms_to_samples(MS)]>,
    index: usize,
    
}

impl<const MS: usize> Delay<MS> {
    pub fn new(length_ms: f32, decay_factor: f32) -> Self {

        let mut buffer = Box::new([(f32, f32); ms_to_samples(MS)]);
        let mut index = 0;

        Delay {
            decay_factor,
            buffer,
        }
    }

    pub fn process(&mut self, input: &mut (f32, f32)) {
        self.buffer[self.index] = *input;

        input.0 = input.0 + (self.decay_factor * self.buffer[self.index + 1].0);
        input.1 = input.1 + (self.decay_factor * self.buffer[self.index + 1].1);

    
        self.clamp(input);

        self.index = (self.index + 1) % ms_to_samples(MS);
    }

    const fn clamp(sound: &mut (f32, f32)) {
        if sound.0 > 1.0 {
            sound.0 = 1.0
        }
        else if sound.0 < -1.0 {
            sound.0 = -1.0
        }

        if sound.1 > 1.0 {
            sound.1 = 1.0
        }
        else if sound.1 < -1.0 {
            sound.1 = -1.0
        }
    }
}

