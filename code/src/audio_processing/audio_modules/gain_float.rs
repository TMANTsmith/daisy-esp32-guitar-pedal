struct Gain_float {
    gain: f32,
}

impl Gain_float {
    fn new(gain: f32) -> Self {
        Self { gain }
    }

    fn process(&self, buffer: &mut [i16]) {
        for sample in buffer.iter_mut() {
            *sample = (*sample as f32 * self.gain)
                .clamp(i16::MIN as f32, i16::MAX as f32) as i16;
        }
    }
}


