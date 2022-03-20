#![no_std]

use embedded_hal::digital::v2::OutputPin;
use fugit::{ExtU32, MicrosDurationU32};
use rust_fsm::*;

state_machine! {
    derive(Debug)
    StepperState(Idle)

    Idle(Start) => StartStepHigh,

    StartStepHigh(PulseStart) => StepHigh,
    StepHigh(PulseEnd) => StartStepLow,

    StartStepLow(PulseStart) => StepLow,
    StepLow(PulseEnd) => StartStepHigh,
}

pub struct A49xx<S, D>
where
    S: OutputPin,
    D: OutputPin, {
    step: S,
    dir: D,

    state_machine: StateMachine<StepperState>
}

impl<S, D> A49xx<S, D>
where
    S: OutputPin,
    D: OutputPin, {
    pub fn new(step: S, dir: D) -> Self {
        Self {
            step, dir,
            state_machine: StateMachine::<StepperState>::new()
        }
    }

    pub fn set_speed(&mut self, _speed: u32) {
        self.state_machine.consume(&StepperStateInput::Start).unwrap();

        // TODO
    }

    pub fn update(&mut self) -> Option<MicrosDurationU32> {
        match *self.state_machine.state() {
            StepperStateState::Idle => { None }
            StepperStateState::StartStepHigh => {
                self.step.set_high().ok();
                self.state_machine.consume(&StepperStateInput::PulseStart).unwrap();

                Some(2_u32.micros()) // TODO
            },
            StepperStateState::StartStepLow => {
                self.step.set_low().ok();
                self.state_machine.consume(&StepperStateInput::PulseStart).unwrap();

                Some(2_u32.millis()) // TODO
            },
            StepperStateState::StepHigh | StepperStateState::StepLow => {
                self.state_machine.consume(&StepperStateInput::PulseEnd).unwrap();

                self.update()
            }
        }
    }
}

