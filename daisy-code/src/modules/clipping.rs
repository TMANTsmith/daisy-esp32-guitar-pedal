use libm;
use crate::modules::process::Effects;

#[derive(Clone)]
pub struct Clipping {
    value: f32,
    bais: f32,
}

impl Clipping {
    pub fn new(value: f32, bais: f32) -> Self {
        Clipping { value, bais }
    }
    pub fn process(&self, input: &mut (f32, f32)) {
        input.0 = libm::tanhf((input.0 + self.bais) * (self.value + 1.0) * (self.value + 1.0));
        input.1 = libm::tanhf((input.1 + self.bais) * (self.value + 1.0) * (self.value + 1.0));
    }
}

impl Effects for Clipping {
    fn process(&mut self, input: &mut f32) {
        *input = libm::tanhf((*input + self.bais) * (self.value + 1.0) * (self.value + 1.0));
    }
}
