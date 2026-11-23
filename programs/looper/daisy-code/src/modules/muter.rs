use crate::modules::share::Module;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::watch::Watch;
use embassy_sync::watch::Receiver;
use embassy_sync::watch::Sender;
use settings::CommandUart;
use settings::MuterCommand;

pub struct Muter {
    active: MuterCommand,
    watch_receiver: Receiver<'static, CriticalSectionRawMutex, MuterCommand, 2>,
}

impl Muter {
    pub fn new() -> (Self, &'static Watch<CriticalSectionRawMutex, MuterCommand, 2>) {
        static WATCH: Watch<CriticalSectionRawMutex, MuterCommand, 2> = Watch::new();
        let watch_receiver = WATCH.receiver().unwrap();


        (Self { active: MuterCommand::Unmute, watch_receiver }, &WATCH) 
    }
}


impl Module for Muter {
    fn process(&mut self, input: &mut f32) {
        if let Some(input) = self.watch_receiver.try_changed() {
            self.active = input;
        }
        if self.active == MuterCommand::Mute {
            *input = 0.0;
        }
    }
}
