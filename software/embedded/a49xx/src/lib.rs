#![no_std]

use embedded_hal::digital::v2::OutputPin;
use fugit::{ExtU32, MicrosDurationU32, HertzU32};
use rust_fsm::*;

state_machine! {
    derive(Debug, PartialEq)
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
    pub pulse_width: MicrosDurationU32,
    step_delay: Option<MicrosDurationU32>,

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
            pulse_width: 2_u32.micros(),
            step_delay: None,
            step, dir,
            state_machine: StateMachine::<StepperState>::new()
        }
    }

    pub fn set_speed(&mut self, speed: HertzU32) {
        if *self.state_machine.state() == StepperStateState::Idle {
            self.state_machine.consume(&StepperStateInput::Start).unwrap();
        }

        self.step_delay = Some(speed.into_duration());
    }

    pub fn update(&mut self) -> Option<MicrosDurationU32> {
        if self.step_delay.is_none() {
            return None;
        }

        match *self.state_machine.state() {
            StepperStateState::Idle => { None }
            StepperStateState::StartStepHigh => {
                self.step.set_high().ok();
                self.state_machine.consume(&StepperStateInput::PulseStart).unwrap();

                Some(self.pulse_width)
            },
            StepperStateState::StartStepLow => {
                self.step.set_low().ok();
                self.state_machine.consume(&StepperStateInput::PulseStart).unwrap();

                Some(self.step_delay.unwrap())
            },
            StepperStateState::StepHigh | StepperStateState::StepLow => {
                self.state_machine.consume(&StepperStateInput::PulseEnd).unwrap();

                self.update()
            }
        }
    }
}

