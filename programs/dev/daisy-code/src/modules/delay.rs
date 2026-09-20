extern crate alloc;
use alloc::boxed::Box;
use crate::modules::process::Effects;

const SAMPLE_RATE: usize = 48000;

pub const fn ms_to_samples(ms: usize) -> usize {
    SAMPLE_RATE * ms / 1000
}



#[derive(Clone)]
pub struct Delay<const T: usize> {
    buffer: [f32; T],
    index: usize,
    decay: f32,
}

impl<const T:usize > Delay<T> {
    pub fn new(decay: f32) -> Self {
        let buffer = [0_f32; T];
        let index = 0;
        Delay {
            buffer,
            decay,
            index,
        }
    }
}

impl<const T: usize>Effects for Delay<T> {
    fn process(&mut self, input: &mut f32) {
        let delayed = self.buffer[self.index];
        
        // Feed back the delayed signal at a reduced level
        self.buffer[self.index] = (*input + delayed * self.decay).clamp(-1.0, 1.0);
        
        self.index = (self.index + 1) % T;
        
        *input = *input + delayed * self.decay;
    }
}

