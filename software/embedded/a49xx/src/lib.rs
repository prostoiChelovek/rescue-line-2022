#![no_std]

use embedded_hal::digital::v2::{OutputPin, PinState};
use fugit::{ExtU32, MicrosDurationU32, HertzU32};
use rust_fsm::*;

#[derive(Debug, PartialEq, Clone,)]
pub enum StepperDireciton {
    Clockwise,
    CounterClockwise
}

pub type SpawnFn = fn() -> ();

state_machine! {
    derive(Debug, PartialEq)
    StepperState(Idle)

    Idle(Start) => StartStepHigh,

    StartStepHigh(PulseStart) => StepHigh,
    StepHigh(PulseEnd) => StartStepLow,

    StartStepLow(PulseStart) => StepLow,
    StepLow(PulseEnd) => StartStepHigh,

    Idle(Stop) => Idle,
    StartStepHigh(Stop) => Idle,
    StepHigh(Stop) => StartStepLow [Stop],
    StartStepLow(Stop) => Idle,
    StepLow(Stop) => Idle,
}

pub struct A49xx<S, D>
where
    S: OutputPin,
    D: OutputPin, {
    pub pulse_width: MicrosDurationU32,
    step_delay: Option<MicrosDurationU32>,

    step: S,
    dir: D,

    state_machine: StateMachine<StepperState>,

    spawn_fn: SpawnFn,

    current_speed: Option<HertzU32>,
    current_direction: Option<StepperDireciton>
}

impl<S, D> A49xx<S, D>
where
    S: OutputPin,
    D: OutputPin, {
    pub fn new(step: S, dir: D, spawn_fn: SpawnFn) -> Self {
        Self {
            pulse_width: 2_u32.micros(),
            step_delay: None,
            step, dir,
            state_machine: StateMachine::<StepperState>::new(),
            spawn_fn,
            current_speed: None,
            current_direction: None
        }
    }

    pub fn set_speed(&mut self, speed: HertzU32) {
        self.step_delay = Some(speed.into_duration());
        self.current_speed = Some(speed);

        if *self.state_machine.state() == StepperStateState::Idle {
            self.state_machine.consume(&StepperStateInput::Start).unwrap();
            (self.spawn_fn)();
        }
    }

    pub fn set_direciton(&mut self, direction: StepperDireciton) {
        // TODO: does not respect timings:
        //       there should be a delay before and after step

        let state = match direction {
            StepperDireciton::Clockwise => { PinState::Low },
            StepperDireciton::CounterClockwise => { PinState::High }
        };
        self.dir.set_state(state).ok();
        self.current_direction = Some(direction);
    }

    pub fn stop(&mut self) {
        self.state_machine.consume(&StepperStateInput::Stop).unwrap();
    }

    pub fn get_speed(&self) -> &Option<HertzU32> { &self.current_speed }

    pub fn get_direction(&self) -> &Option<StepperDireciton> { &self.current_direction }

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

