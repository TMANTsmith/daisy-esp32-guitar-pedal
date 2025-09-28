struct Gain_float {
    gain: f32,
}

impl Gain_float {
    fn new(gain: f32) -> Self {
        Self { gain }
    }

    fn process(&self, buffer: &mut [f32]) {
        for sample in buffer.iter_mut() {
            *sample = (*sample* self.gain)
                .clamp(f32::MIN, f32::MAX)
        }
    }
}


