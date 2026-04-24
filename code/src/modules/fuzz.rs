pub struct Fuzz {
    value: f32,
}

impl Fuzz {
    pub fn new(value: f32) -> Self {
        let mut vin = value - 1.0;
        if vin < 0.0 { 
            vin = -vin; 
        }
        Fuzz {value : vin}
    }
    pub fn process(&self, input: &mut (f32, f32)) {
        if input.0 > self.value {
            input.0 = self.value;
        } else if input.0 < -self.value {
            input.0 = -self.value;
        }

        if input.1 > self.value {
            input.1 = self.value;
        } else if input.1 < -self.value {
            input.1 = -self.value;
        }

        input.0 = input.0 * (1.0 / self.value);
        input.1 = input.1 * (1.0 / self.value);
    }

}
