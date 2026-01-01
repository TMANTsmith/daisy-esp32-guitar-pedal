use libm::roundf; // for f32
use core::num::Wrapping;



pub struct Delay<'a> {
    volume: f32,          // feedback/mix
    buffer: &'a mut [(f32, f32)],
    write_idx: Wrapping<usize>,
    length_samples: usize,
}

impl<'a> Delay<'a> {
    pub fn new(length_ms: usize, volume: f32, buffer: &'a mut [(f32, f32)], sample_rate: f32) -> Self {
        let length_samples = roundf((length_ms as f32) * sample_rate / 1000.0) as usize;
        Delay {
            volume,
            buffer,
            write_idx: Wrapping(0),
            length_samples,
        }
    }

    pub fn process(&mut self, input: &mut (f32, f32)) {
        let buf_len = self.buffer.len();

        // read index based on delay length in samples
        let read_idx = (self.write_idx.0 + buf_len - self.length_samples) % buf_len;
        let (left, right) = self.buffer[read_idx];

        // mix delayed sample into input
        *input = (
            input.0 + left * self.volume,
            input.1 + right * self.volume,
        );

        // write current input to buffer
        self.buffer[self.write_idx.0] = *input;

        // advance write index with wrap-around
        self.write_idx += Wrapping(1);
        if self.write_idx.0 >= buf_len {
            self.write_idx = Wrapping(0);
        }
    }

    pub fn process_list(&mut self, input: &mut [(f32, f32)]) {
        for sample in input.iter_mut() {
            self.process(sample);
        }
    }
}

