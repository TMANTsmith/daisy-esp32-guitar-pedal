pub struct Stereo<E> {
    left: E,
    right: E,
}

impl<E: Effect> Stereo<E> {
    pub fn new(effect: E) -> Self {
        left = effect.clone();
        right = effect;
        Self { left, right }
    }

    pub fn process(input: (f32, f32)) {
        left.process(input.0);
        right.process(input.1);
    }
}
