use diasy;
use embedded_hal::digital::v2::InputPin;

enum State {
    Idle,
    Recording,
    Replaying,
    OverDubbing,
}

pub struct Looper<P: InputPin> {
    buffer: Vec<(f32, f32)>,
    pin: P,
    index: usize,
    state: State,
}

impl<P: InputPin> Looper<P> {
    pub fn new(pin: &P) -> Self {
        let mut buffer: Vec<(f32, f32)> = Vec::new();
        let mut index = 0;
        let mut state = State::Idle;
        Self { buffer, pin, index , state }
    }
    pub fn is_on(&self) -> bool {
        self.pin.is_high().unwrap_or(false)
    }


    pub fn process(&self, input: &mut (f32, f32)) {
        if is_on() {
            vec.push(*input);
        }
        else {
            input =vec[self.index];
        }
}






