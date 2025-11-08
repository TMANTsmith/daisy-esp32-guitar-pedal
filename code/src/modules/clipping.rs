struct Clipping {
    value: f64,
    bais: f64,
}

impl Clipping {
    fn new(value: f64, bais: f64) -> Self {
        Fuzz { value, bais }
    }

    fn process(&self, input: &mut (f64, f64)) {
        input.0 = ((input.0 + self.bais) * (self.value+ 1.0) * (self.value + 1.0)).tanh();
        input.1 = ((input.1 + self.bais) * (self.value+ 1.0) * (self.value + 1.0)).tanh();
    }

    fn process_list(&self, input: &mut [(f64, f64)]) {
        for tuple in input.iter_mut() {
            self.process(tuple);
        }
    }
}
