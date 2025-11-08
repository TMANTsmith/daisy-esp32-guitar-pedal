struct Fuzz {
    fuzz: f64,
    bais: f64,
}

impl Fuzz {
    fn new(fuzz: f64, bais: f64) -> Self {
        Fuzz { fuzz, bais }
    }

    fn process(&self, input: &mut (f64, f64)) {
        input.0 = ((input.0 + self.bais) * (self.fuzz + 1.0) * (self.fuzz + 1.0)).tanh();
        input.1 = ((input.1 + self.bais) * (self.fuzz + 1.0) * (self.fuzz + 1.0)).tanh();
    }

    fn process_list(&self, input: &mut [(f64, f64)]) {
        for tuple in input.iter_mut() {
            self.process(tuple);
        }
    }
}
