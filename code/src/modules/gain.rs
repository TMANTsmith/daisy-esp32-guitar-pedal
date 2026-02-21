
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
