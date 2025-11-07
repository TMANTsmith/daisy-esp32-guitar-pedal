struct Gain {
    value: u8,
}

impl Gain {
    fn new(value: u8) -> Self {
        Gain { value }
    }

    fn set_value(&mut self, value: u8) {
        self.value = value;
    }

    fn get_value(&self) -> u8 {
        self.value
    }
    fn process(&self, input: &mut (f64, f64)) {
        *input = ((input.0 * self.value as f64), (input.1 * self.value as f64))
    }
    fn process_list(&self, input: &mut [(f64, f64); 4]) {
        for touple in input.iter_mut() {
            self.process(touple);
        }
    }
}
    

