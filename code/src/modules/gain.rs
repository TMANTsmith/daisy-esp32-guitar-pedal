
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
    pub fn process(&self, input: &mut (f32, f32)) {
        *input = ((input.0 * self.value as f32), (input.1 * self.value as f32))
    }
}
