use alloc::vec;
use daisy_embassy::hal::exti::ExtiInput;
use daisy_embassy::hal::usart::UartRx;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use daisy_embassy::hal::{bind_interrupts, exti, peripherals};
use daisy_embassy::hal::interrupt::typelevel::EXTI0;
use daisy_embassy::hal::mode::Async;
use embassy_time::{ Duration, Instant };
use modular_bitfield::prelude::*;
use alloc::vec::Vec;
use pcobs::{serialize, deserialize};
use embassy_sync::mutex::Mutex;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use static_cell::StaticCell;

bind_interrupts!(struct Irqs {
    EXTI0 => exti::InterruptHandler<EXTI0>;
});



    static STATE: Mutex<ThreadModeRawMutex, State> = Mutex::new(
        State::new()
        .with_mode(TrackState::Idle)
        .with_track_number(0)
    );

#[embassy_executor::task]
async fn uart_receiver(mut uart: UartRx<'static, Async>) {
    let mut buf = [0; 64];
    let mut counter = 0usize;
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

            match deserialize::<State>(frame, frame.len()) {
                Ok(result) => {
                    let mut state = STATE.lock().await;
                    *state = result;

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
async fn button_detector(mut pin: ExtiInput<'static, Async>) {
    loop {
        pin.wait_for_rising_edge().await;
        let time = Instant::now();
        {
        let mut state = STATE.lock().await;
        let result = match state.mode() {
            TrackState::Idle => TrackState::Recording,
            TrackState::Recording => TrackState::Idle,
            TrackState::Clear => TrackState::Clear,
        }
        state.set_mode(result);
        }
        
        pin.wait_for_falling_edge().await;
        let held_for = time.elapsed().as_millis();

        if held_for >= 1000 {
            STATE.lock().await.set_mode(TrackState::Clear);
        }
    }
}

type Frame = (f32, f32);
type Track = Vec<Frame>;


#[bitfield]
#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct State {
    track_number: B6,
    mode: TrackState,
}

#[derive(Specifier, Debug, PartialEq, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[bits = 2]
pub enum TrackState {
    Idle,
    Recording,
    Clear,
}


pub struct Looper {
    tracks: Vec<Track>,
    index: usize,
    state: State,
}

pub struct Second(usize);

impl Looper {
    pub fn new(tracks_count: usize, time: Second) -> Self {
        let state = State::Idle;

        let mut tracks: Vec<Track> = Vec::with_capacity(tracks_count); 
        let length = time.0 * 48_000;
        for i in 0..tracks_count {
            let track = vec![(0.0, 0.0); length];
            tracks.push(track);
        }
        Self { tracks, index: 0, state }
    }
    fn add_tuple(a: &mut Frame, b: &Frame) {
        a.0 += b.0;
        a.1 += b.1;
    }
    fn add_all_list(input: Vec<Track>, index: usize) -> Frame {
        let mut output: (f32, f32) = (0.0, 0.0);
        for track in input.iter() {
            let frame = &track[index];
            Self::add_tuple(&mut output, frame);
        }
        output
    }
    fn clamp_tuple(input: &mut Frame) {
        input.0.clamp(-1.0, 1.0);
        input.1.clamp(-1.0, 1.0);
    }
    fn advance(&mut self) {
        self.index = (self.index + 1) % self.buffer.len();
    }

    pub async fn process(&mut self, input: &mut Frame) {
        let result: State = STATE.lock().await.copy();
        match result.mode() {
            TrackState::Idle => {
                Self::add_tuple(input, &Self::add_all_list(self.tracks, self.index));
                self.advance();
            }
            TrackState::Recording => {
                self.tracks[self.state.track_number()][self.index] = *input;
            }
            TrackState::Clear => {
                self.tracks[self.state.track_number()].fill((0.0, 0.0));
            }
        }
    }
}
