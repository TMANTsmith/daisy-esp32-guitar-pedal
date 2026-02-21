use libm;

pub struct Clipping {
    value: f32,
    bais: f32,
}

impl Clipping {
    pub fn new(value: f32, bais: f32) -> Self {
        Clipping { value, bais }
    }
}
