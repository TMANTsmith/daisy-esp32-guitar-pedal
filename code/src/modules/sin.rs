use libm::sinf;
const PI: f32 = 3.1415927410e+00;
const SAMPLE: f32 = 48_000.0;

pub struct Sine {
    phase: f32,
    phase_inc: f32,
    amplitude: f32,
}

impl Sine {
    pub fn new(frequency: f32, amplitude: f32) -> Self {
        Self {
            phase: 0.0,
            phase_inc: frequency * 2.0 * PI / SAMPLE,
            amplitude,
        }
    }

    pub fn get_next(&mut self) -> (f32, f32) {
    let f = self.phase;
    
    let wave = (
        sinf(f)           * 1.0 +
        sinf(f * 2.0)     * 0.5 +
        sinf(f * 3.0)     * 0.33 +
        sinf(f * 4.0)     * 0.25 +
        sinf(f * 5.0)     * 0.2
    ) / 2.28; // normalize by sum of coefficients
    
    let wave = wave * self.amplitude;

    self.phase += self.phase_inc;
    if self.phase >= 2.0 * PI {
        self.phase -= 2.0 * PI;
    }
    (wave, wave)
}
}
