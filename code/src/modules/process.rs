// TODO add Delay



use crate::modules::gain::Gain;
use crate::modules::fuzz::Fuzz;
use crate::modules::clipping::Clipping;
use crate::modules::bit_crush::BitCrush;


pub enum Effects {
    Gain(Gain),
    Fuzz(Fuzz),
    Clipping(Clipping),
    BitCrush(BitCrush),
}

pub fn process_all(input: &mut (f32, f32), effects: &[Effects] ) {
    for effect in effects {
        effect.process(input)
    }
}
impl Effects {
    fn process(&self, input: &mut (f32, f32)) {
        match self {
            Effects::Gain(e) => { e.process(input) },
            Effects::Fuzz(e) => { e.process(input) },
            Effects::Clipping(e) => { e.process(input) },
            Effects::BitCrush(e) => { e.process(input) },
        }
    }
}


