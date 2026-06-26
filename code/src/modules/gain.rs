
use crate::modules::process::{self, Effects};
pub struct Gain {
    value: f32,
}

impl Gain {
    pub fn new(value: f32) -> Self {
        Gain { value }
    }

    pub fn update(&mut self, value: f32) {
        self.value = value;
    }
}

impl Effects for Gain {
    fn process(&mut self, input: &mut f32) {
        *input = (*input * self.value).clamp(-1.0, 1.0)
    }
}
