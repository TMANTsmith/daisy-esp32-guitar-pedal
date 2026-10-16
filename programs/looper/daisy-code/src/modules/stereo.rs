pub struct Stereo<E> {
    left: E,
    right: E,
}


impl<E: Effect> Stereo<E> {
    pub fn new(effect: E) -> Self {
        let left = effect.clone();
        let right = effect;
        Self { left, right }
    }

    pub fn process(&mut self, input: (f32, f32)) {
        self.left.process(input.0);
        self.right.process(input.1);
    }
}
