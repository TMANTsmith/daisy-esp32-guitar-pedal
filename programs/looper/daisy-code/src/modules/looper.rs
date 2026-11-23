use core::ops::Index;

use alloc::vec;
use daisy_embassy::hal::exti::ExtiInput;
use daisy_embassy::hal::pac::rtc::regs::Tr;
use daisy_embassy::hal::usart::UartRx;
use daisy_embassy::hal::usart::UartTx;
use daisy_embassy::hal::mode::Async;
use defmt::Debug2Format;
use embassy_sync::signal::Signal;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::watch::{Receiver, Sender, Watch, AnonReceiver};
use embassy_sync::blocking_mutex::CriticalSectionMutex;
use embassy_time::{ Duration, Instant };
use modular_bitfield::prelude::*;
use alloc::vec::Vec;
use pcobs::{serialize, deserialize};
use embassy_sync::mutex::Mutex;
use daisy_embassy::led::UserLed;
use static_cell::StaticCell;
use defmt::info;
use settings::*;
use alloc::boxed::Box;

use crate::modules::share::Module;

#[embassy_executor::task]
pub async fn mange_commands(
    uart_receiver: Signal<CriticalSectionRawMutex, CommandUart>,
    uart_sender: Signal<CriticalSectionRawMutex, CommandUart>,
    button: Signal<CriticalSectionRawMutex, CommandUart>,
    mute_sender: Sender<'static, CriticalSectionRawMutex, MuterCommand, 2>,
    looper_sender: Sender<'static, CriticalSectionRawMutex, LooperCommand, 2>,
) {


}



#[embassy_executor::task]
pub async fn uart_receiver(mut uart: UartRx<'static, Async>, 
    mut led: UserLed<'static>, 
    mute_sender: Sender<'static, CriticalSectionRawMutex, MuterCommand, 2>,
    looper_sender: Sender<'static, CriticalSectionRawMutex, LooperCommand, 2>) {
    info!("uart receiver started");
    let mut buf = Box::new([0; 32]);
    let mut counter = 0usize;
    let mut is_on = false;
    loop {
        let n = match uart.read_until_idle(&mut buf[counter..]).await {
            Err(e) => { info!("uart read error: {}", e); continue; }
            Ok(n) => n,
        };

        let end = counter + n; 
        let mut start = 0;

        while let Some(rel_pos) = buf[start..end].iter().position(|&b| b == FRAME_DELIM) {
            let delim = start + rel_pos;
            let frame = &mut buf[start..delim]; 

            info!("try to deserialize: {}", frame);

            match deserialize::<CommandUart>(frame, frame.len()) {
                Ok(result) => {
                    if is_on {
                        led.off();
                        is_on = false;
                    }
                    else {
                        led.on();
                        is_on = true;
                    }
                    info!("read correct: {:?}", defmt::Debug2Format(&result));
                    match result {
                        CommandUart::Muter(c) => mute_sender.send(c),
                        CommandUart::Looper(c) => looper_sender.send(c),
                    }
                }
                Err(e) => info!("bad cobs frame {}", defmt::Debug2Format(&e)),
            }

            start = delim + 1;
        }

        if start == end {
            // consumed everything, reset to front of buffer
            counter = 0;
        } else if start > 0 {
            // shift leftover partial frame to the front of buf
            buf.copy_within(start..end, 0);
            counter = end - start;
        } else {
            // no delimiter found at all — still mid-frame, keep accumulating
            counter = end;
            if counter == buf.len() {
                info!("frame too large, dropping");
                counter = 0;
            }
        }
    }
}

#[embassy_executor::task]
pub async fn uart_sender(mut uart: UartTx<'static, Async>, signal_receiver: &'static Signal<CriticalSectionRawMutex, CommandUart>) {
    loop {
        let mut buf = Box::new([0; 32]);
        let command = signal_receiver.wait().await;
        let len = match serialize(&command, buf.as_mut_slice()) {
            Ok(l) => {
                buf[l] = FRAME_DELIM;
                l + 1
            },
            Err(err) => {
                //info!("error: {}", Debug2Format(&err));
                continue;
            }
        };

        uart.write(&buf[..=len]).await.unwrap();
    }
}

    
#[embassy_executor::task]
pub async fn button_detector(watch_sender: Sender<'static, CriticalSectionRawMutex, CommandUart, 2>, signal_sender: &'static Signal<CriticalSectionRawMutex, CommandUart>, mut pin: ExtiInput<'static, Async>) {
    loop {
        pin.wait_for_rising_edge().await;
        let time = Instant::now();
        embassy_time::Timer::after_millis(10).await;
        if let Some(mut command) = watch_sender.try_get() {
            match &mut command {
                CommandUart::Looper(mut state) => {
                    let result = match state.state() {
                        TrackStateCommand::Play => TrackStateCommand::Record,
                        TrackStateCommand::Record => TrackStateCommand::Play,
                        TrackStateCommand::Clear => TrackStateCommand::Clear,
                    };
                    state.set_state(result);
                    watch_sender.send(command);
                    info!("button pressed");
                    signal_sender.signal(command);

                    pin.wait_for_falling_edge().await;
                    let held_for = time.elapsed().as_millis();

                    if held_for >= 1000 {
                        state.set_state(TrackStateCommand::Clear);
                        watch_sender.send(command);
                        signal_sender.signal(command);
                    }
                }
                CommandUart::Muter(mut state) => {
                    match state {
                        MuterCommand::Mute => state = MuterCommand::Unmute,
                        MuterCommand::Unmute => state = MuterCommand::Mute,
                    }
                    watch_sender.send(command);
                    signal_sender.signal(command);
                }
            }
        }
    }
}

pub struct Second(pub usize);

#[derive(Clone)]
struct Track {
    /// This is the state of the Track
    state: TrackState,
    /// This is the buffer that holds the recorded sections
    values: Vec<f32>,
    /// This is the length of the section that is recorded
    recording_length: usize,
    /// This is the active index being read
    index: usize
}

// make a function that will read value of values but will not increment the index
impl Track {
    fn new(length: usize) -> Self {
        Track { 
            state: TrackState::Idling, 
            values: vec![0.0; length], 
            recording_length: 0,
            index: 0,
        }
    }
    pub fn state(&self) -> TrackState {
        self.state
    }
    pub fn record_point(&mut self, input: &f32) {
        self.values[self.index] = *input;
    }
    pub fn overdub_point(&mut self, input: &f32) {
        self.values[self.index] += *input;
    }

    pub fn send_command(&mut self, state: TrackStateCommand) {
        info!("track got command");
        match state {
            TrackStateCommand::Clear => self.state = TrackState::Idling,
            TrackStateCommand::Record => {
                if self.state == TrackState::Idling {
                    self.state = TrackState::Recording;
                }
                else if self.state == TrackState::Replaying  {
                    self.state = TrackState::Overdubbing;
                }
                else if self.state == TrackState::Playing {
                    self.state = TrackState::Overdubbing;
                }
            }
            TrackStateCommand::Play => {
                if self.state == TrackState::Idling {
                    self.state = TrackState::Playing;
                }
                else if self.state == TrackState::Recording {
                    self.state = TrackState::Playing;
                    self.recording_length = self.index;
                }
                else if self.state == TrackState::Overdubbing {
                    self.state = TrackState::Replaying;
                    self.recording_length = self.index;
                }
            }
        }
    }
}
impl Iterator for Track {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.values.is_empty() {
            return None;
        }

        let mut sample: Option<f32> = None;

        match self.state {
            TrackState::Idling => { 
                sample = None;
            }
            TrackState::Playing => { 
                if self.index >= self.recording_length {
                    self.index = 0;
                }
                else {
                    sample = Some(self.values[self.index]);
                    self.index += 1;
                }
            }
            TrackState::Replaying => { 
                if self.index >= self.recording_length {
                    self.index = 0;
                }
                else {
                    sample = Some(self.values[self.index]);
                    self.index += 1;
                }
            }
            TrackState::Overdubbing => {
                if self.index >= self.values.len() {
                    sample = None;
                }
                else {
                    sample = Some(self.values[self.index]);
                    self.index += 1;
                }
            }
            TrackState::Recording => {
                if self.index >= self.values.len() {
                    sample = None;
                }
                else {
                    sample = Some(self.values[self.index]);
                    self.index += 1;
                }
            }
        }
        sample
    }
}

impl Index<usize> for Track {
    type Output = f32;
    fn index(&self, index: usize) -> &Self::Output {
        &self.values[index]
    }
}



pub struct Looper {
    tracks: Vec<Track>,
    watch_receiver: Receiver<'static, CriticalSectionRawMutex, LooperCommand, 2>,
}

impl Looper {
    pub fn new(tracks_count: usize, time: Second)-> (Self, &'static Watch<CriticalSectionRawMutex, LooperCommand, 2>) {
        static WATCH: Watch<CriticalSectionRawMutex, LooperCommand, 2> = Watch::new();

        let track_capacity = time.0 * SAMPLE_RATE;
        let watch_receiver = WATCH.receiver().unwrap();

        (Self {
            tracks: vec![Track::new(track_capacity); tracks_count],
            watch_receiver,
        }, &WATCH)
    }
    pub fn send_command(&mut self, command: LooperCommand) {
        let track_number = command.track_number();
        let track: &mut Track = &mut self.tracks[track_number as usize];
        track.send_command(command.state());
    }

}

impl Module for Looper {
    fn process(&mut self, input: &mut f32) {
        // Read the command once. If Clear is received, immediately change
        // the shared command to Idle, but keep Clear locally for this sample.
        if let Some(command) = self.watch_receiver.try_changed() {
            self.send_command(command);
        }
        // this is so that overdubs don't record the other tracks
        let mut buffer = *input;
        for track in self.tracks.iter_mut() {
            match track.state() {
                TrackState::Idling => {},
                TrackState::Playing => { buffer += track.next().unwrap_or(0.0);  },
                TrackState::Replaying => { buffer += track.next().unwrap_or(0.0);  },
                TrackState::Recording => { track.record_point(input); track.next();  },
                TrackState::Overdubbing => { track.overdub_point(input); buffer += track.next().unwrap_or(0.0); }
            }
        }
        *input = buffer;
    }
}
