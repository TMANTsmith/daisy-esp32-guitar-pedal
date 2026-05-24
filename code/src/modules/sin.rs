use libm::sinf;
const PI: f32 = 3.1415927410e+00;
const SAMPLE: f32 = 48_000.0;

pub struct Sine {
    sample: u32,
    frequency: f32,
    amplitude: f32,
}
impl Sine {
    pub fn new(sample: u32, frequency: f32, amplitude: f32) -> Self {
        Self {
            sample,
            frequency,
            amplitude,
        }
    }

    pub fn get_next(&mut self) -> (f32, f32) {
        let t = self.sample as f32 / SAMPLE;
        let wave = self.amplitude * sinf(self.frequency * 2_f32 * PI * t);
        self.sample = self.sample.wrapping_add(1);
        (wave, wave)
    }
}
