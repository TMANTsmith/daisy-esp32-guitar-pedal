use libm::sinf;
const PI: f32 = 3.1415927410e+00;
const SAMPLE: f32 = 48_000.0;


pub struct Sine {
    time: f32,
    frequency: f32,
    amplitude: f32,

}
impl Sine {
    fn new(time: f32, frequency: f32, amplitude: f32) -> Self {
        Self { time, frequency, amplitude }
    }

    fn get_next(&mut self) -> (f32, f32) {
        let wave = self.amplitude * sinf(self.frequency * (2_f32 * PI) * self.time);
        self.time += 1_f32 / SAMPLE;

        (wave, wave)

    }
}
