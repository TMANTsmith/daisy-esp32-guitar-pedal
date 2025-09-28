struct Gain_int {
    gain: i16,
}

impl Gain_int {
    fn new(gain: i16) -> Self {
        Self { gain }
    }

    fn process(&self, buffer: &mut [i16]) {
        for sample in buffer.iter_mut() {
            *sample = (*sample self.gain)
                .clamp(i16::MIN , i16::MAX );
        }
    }
}


