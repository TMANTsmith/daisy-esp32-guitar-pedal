use embassy_sync::watch::Watch;
use embassy_sync::watch::Sender;
use embassy_sync::blocking_mutex::raw::RawMutex;
use core::clone::Clone;

pub struct Stereo<M> {
    left: M,
    right: M,
}

pub trait Module {
    fn process(&mut self, input: &mut f32);
}


/*
impl<M: Module> Stereo<M> {
    pub fn new(effect: M) -> Self {
        let left = effect;
        let right = effect;
        Self { left, right }
    }
}

*/
