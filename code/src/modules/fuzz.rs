struct Fuzz {
    value: f64,
}

impl Fuzz {
    fn new(value: f64) -> Self {
        let mut vin = value - 1.0;
        if vin < 0.0 { 
            vin = -vin; 
        }
        Fuzz {value : vin}
    }

    fn process(&self, input: &mut (f64, f64)) {

        if input.0 > self.value {
            input.0 = self.value;
        } 
        else if input.0 < -self.value {
            input.0 = -self.value;
        }
        
        if input.1 > self.value {
            input.1 = self.value;
        } 
        else if input.1 < -self.value {
            input.1 = -self.value;
        }

        input.0 = input.0 * (1.0 / self.value);
        input.1 = input.1 * (1.0 / self.value);

    }

    fn process_list(&self, input: &mut [(f64, f64)]) {
        for tuple in input.iter_mut() {
            self.process(tuple);
        }
    }
}
