use daisy_embassy::hal::exti::ExtiInput;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use daisy_embassy::hal::gpio::Pin;
use daisy_embassy::hal::gpio::Input;
use daisy_embassy::hal::{bind_interrupts, exti, peripherals};
use daisy_embassy::hal::interrupt::typelevel::EXTI0;
use daisy_embassy::hal::mode::Async;
use embassy_time::{ Duration, Instant };
use alloc::vec::Vec;

bind_interrupts!(struct Irqs {
    EXTI0 => exti::InterruptHandler<EXTI0>;
});


static STATE: Signal<CriticalSectionRawMutex, State> = Signal::new();

#[embassy_executor::task]
async fn uart_sender(mut pin: ExtiInput<'static, Async>) {
    let mut state = State::Idle;
    loop {
        pin.wait_for_rising_edge().await;
        let time = Instant::now();
        match state {
            State::Idle => { state = State::Recording },
            State::Recording => { state = State::Replaying },
            State::Replaying => { state = State::OverDubbing },
            State::OverDubbing => { state = State::Replaying },
            State::Clear => { state = State::Idle },
        }
        STATE.signal(state);
        pin.wait_for_falling_edge().await;
        let held_for = time.elapsed().as_millis();

        if held_for >= 1000 {
            state = State::Idle;
            STATE.signal(State::Clear);
        }

    }


}

type Frame = (f32, f32);

#[derive(Clone, Copy, PartialEq)]
enum State { Idle, Recording, Replaying, OverDubbing, Clear }

pub struct Looper {
    buffer: Vec<Frame>,
    index: usize,
    state: State,
}

impl Looper {
    pub fn new(buffer: Vec<Frame>) -> Self {
        let state = State::Idle;
        Self { buffer, index: 0, state }
    }
    fn add_tuple(a: &mut Frame, b: &Frame) {
        a.0 += b.0;
        a.1 += b.1;
    }

    fn advance(&mut self) {
        self.index = (self.index + 1) % self.buffer.len();
    }

    pub fn process(&mut self, input: &mut Frame) {
        if let Some(state) = STATE.try_take() {
            self.state = state;
        }
        match self.state {
            State::Idle => {}
            State::Recording => {
                self.buffer.push(*input);
            }
            State::Replaying => {
                Self::add_tuple(input, &self.buffer[self.index]);
                self.advance();
            }
            State::OverDubbing => {
                let dry: Frame = *input;
                Self::add_tuple(input, &self.buffer[self.index]);   // hear the loop
                Self::add_tuple(&mut self.buffer[self.index], &dry);      // add new material
                self.advance();
            }
            State::Clear => {
                self.buffer.fill((0.0, 0.0));
                self.state = State::Idle;
            }
        }
    }
}
