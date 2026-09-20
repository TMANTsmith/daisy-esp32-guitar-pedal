use crate::modules::process::Effects;

#[derive(Clone)]
pub struct Fuzz {
    value: f32,
}

impl Fuzz {
    pub fn new(value: f32) -> Self {
        Fuzz {value }
    }
}

impl Effects for Fuzz {
    fn process(&mut self, input: &mut f32) {
        if *input > self.value {
            *input = self.value;
        } else if *input < -self.value {
            *input = -self.value;
        }

        *input *= 1.0 / self.value;
    }
    // add code here
}
