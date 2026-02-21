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
}
