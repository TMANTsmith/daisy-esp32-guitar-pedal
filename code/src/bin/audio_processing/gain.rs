struct Gain {
    gain: f32,
}

impl Gain{
    fn new(gain: f32) -> Self {
        Self { gain }
    }

    fn process(&self, sample: f32) -> f32 {
        (sample * self.gain).clamp(f32::-1.0, f32::1.0)
    }
}



