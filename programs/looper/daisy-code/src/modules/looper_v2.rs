use alloc::vec;
use daisy_embassy::hal::exti::ExtiInput;
use daisy_embassy::hal::usart::UartRx;
use daisy_embassy::hal::mode::Async;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
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




    pub static COMMAND_UART: Mutex<CriticalSectionRawMutex, CommandUart> = Mutex::new(
        CommandUart::Looper(State::new())
    );

#[embassy_executor::task]
pub async fn uart_receiver(mut uart: UartRx<'static, Async>, mut led: UserLed<'static>) {
    info!("uart receiver started");
    let mut buf = Box::new([0; 64]);
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
                    let mut state = COMMAND_UART.lock().await;
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
pub async fn button_detector(mut pin: ExtiInput<'static, Async>) {
    loop {
        pin.wait_for_rising_edge().await;
        let time = Instant::now();
        let mut state = {
            let command = COMMAND_UART.lock().await;
            *command
        };
        match &mut state {
            CommandUart::Looper(mut state) => {
                let result = match state.mode() {
                    TrackState::Idle => TrackState::Recording,
                    TrackState::Recording => TrackState::Idle,
                    TrackState::Clear => TrackState::Clear,
                };
                state.set_mode(result);

                pin.wait_for_falling_edge().await;
                let held_for = time.elapsed().as_millis();

                if held_for >= 1000 {
                    state.set_mode(TrackState::Clear);
                }
            }
            CommandUart::Muter(mut state) => {
                match state {
                    Mute::Mute => state = Mute::Unmute,
                    Mute::Unmute => state = Mute::Mute,
                }
            }
        }
    }
}

type Track = Vec<f32>;
pub struct Second(pub usize);

pub struct Looper {
    tracks: Vec<Track>,
    state: State,
    index: Vec<usize>,
    recorded_length: Vec<usize>,
    recording_limit: Vec<usize>,
}

impl Looper {
    pub fn new(tracks_count: usize, time: Second) -> Self {
        let track_capacity = time.0 * SAMPLE_RATE;

        Self {
            tracks: vec![vec![0.0; track_capacity]; tracks_count],
            state: State::new(),
            index: vec![0; tracks_count],
            recorded_length: vec![0; tracks_count],
            recording_limit: vec![0; tracks_count],
        }
    }

    fn add_all_list(&self) -> f32 {
        let mut output = 0.0;

        for (track_number, track) in self.tracks.iter().enumerate() {
            let length = self.recorded_length[track_number];

            // Empty tracks contribute nothing and must stay at index 0.
            if length == 0 {
                continue;
            }

            output += track[self.index[track_number]];
        }

        output
    }

    fn advance_playback(&mut self, track_number: usize) {
        let length = self.recorded_length[track_number];

        if length == 0 {
            self.index[track_number] = 0;
            return;
        }

        self.index[track_number] =
            (self.index[track_number] + 1) % length;
    }

    fn advance_recording(&mut self, track_number: usize) {
        let limit = self.recording_limit[track_number];

        let length = if limit > 0 {
            limit
        } else {
            self.tracks[track_number].len()
        };

        if length > 0 {
            self.index[track_number] =
                (self.index[track_number] + 1) % length;
        } else {
            self.index[track_number] = 0;
        }
    }

    fn reset_empty_track_indices(&mut self) {
        for track_number in 0..self.tracks.len() {
            if self.recorded_length[track_number] == 0 {
                self.index[track_number] = 0;
                self.recording_limit[track_number] = 0;
            }
        }
    }

pub fn process(&mut self, input: &mut f32) {
    // Read the command once. If Clear is received, immediately change
    // the shared command to Idle, but keep Clear locally for this sample.
    // If the mutex is busy, retain self.state and continue audio processing.
    let command = if let Ok(mut command) = COMMAND_UART.try_lock() {
        match *command {
            CommandUart::Looper(state)
                if state.mode() == TrackState::Clear =>
            {
                *command = CommandUart::Looper(
                    State::new()
                        .with_track_number(state.track_number())
                        .with_mode(TrackState::Idle),
                );

                Some(CommandUart::Looper(state))
            }
            other => Some(other),
        }
    } else {
        None
    };
    // Mutex guard is dropped here.

    // Apply a newly received command, if one was available.
    // Otherwise, continue using the previous self.state.
    if let Some(CommandUart::Looper(new_state)) = command {
        if new_state != self.state {
            let track_number = new_state.track_number() as usize;

            if track_number < self.tracks.len() {
                if new_state.mode() == TrackState::Recording {
                    let existing_length = self.recorded_length[track_number];

                    // Limit overdubbing to the existing recording length.
                    self.recording_limit[track_number] = existing_length;

                    if existing_length > 0 {
                        self.index[track_number] %= existing_length;
                    } else {
                        self.index[track_number] = 0;
                    }
                }

                self.state = new_state;
            }
        }
    }

    let track_number = self.state.track_number() as usize;

    if track_number >= self.tracks.len() {
        self.reset_empty_track_indices();
        return;
    }

    match self.state.mode() {
        TrackState::Idle => {
            // Mix all non-empty tracks.
            *input = (*input + self.add_all_list()).clamp(-1.0, 1.0);

            // Advance only tracks that contain recorded audio.
            for track_number in 0..self.tracks.len() {
                self.advance_playback(track_number);
            }
        }

        TrackState::Recording => {
            let limit = self.recording_limit[track_number];

            if limit > 0 {
                // Overdub the existing recording.
                let index = self.index[track_number];

                let sample = (
                    self.tracks[track_number][index] + *input
                )
                .clamp(-1.0, 1.0);

                self.tracks[track_number][index] = sample;
                *input = sample;
            } else {
                // Record a new take.
                let index = self.index[track_number];

                if index < self.tracks[track_number].len() {
                    self.tracks[track_number][index] = *input;

                    self.recorded_length[track_number] =
                        self.recorded_length[track_number].max(index + 1);
                } else {
                    // Prevent out-of-bounds access.
                    self.index[track_number] = 0;
                }
            }

            self.advance_recording(track_number);
        }

        TrackState::Clear => {
            // Clear the selected track once.
            self.tracks[track_number].fill(0.0);
            self.recorded_length[track_number] = 0;
            self.index[track_number] = 0;
            self.recording_limit[track_number] = 0;

            // The shared command was already changed to Idle while locked.
            // Update our local state so the track is not cleared repeatedly.
            self.state = State::new()
                .with_track_number(track_number as u8)
                .with_mode(TrackState::Idle);
        }
    }

    // Keep empty-track indices at zero.
    self.reset_empty_track_indices();
}
}


