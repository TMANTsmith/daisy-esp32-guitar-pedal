pub trait Effects {
    fn process(&mut self, input: &mut f32);
}

// not implmented for FFT
