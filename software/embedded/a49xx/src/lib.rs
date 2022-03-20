#![no_std]

use embedded_hal::digital::v2::OutputPin;
use fugit::{ExtU32, MicrosDurationU32, HertzU32};
use rust_fsm::*;

pub type SpawnFn = fn() -> ();

pub struct StepperTimings {
    pub pulse_width: MicrosDurationU32,
}

impl Default for StepperTimings {
    fn default() -> Self {
        Self {
            pulse_width: 1_u32.micros()
        }
    }
}

state_machine! {
    derive(Debug, PartialEq)
    StepperState(Idle)

    Idle(Start) => StartStepping,
    StartStepping(Started) => Stepping,
    Idle(Stop) => Idle,

    Stepping(Stop) => StopStepping,
    StopStepping(Stopped) => Idle
}

state_machine! {
    derive(Debug, PartialEq)
    StepState(Idle)

    Idle(Start) => StartHigh,

    StartHigh(PulseStart) => High,
    High(PulseEnd) => StartLow,

    StartLow(PulseStart) => Low,
    Low(PulseEnd) => StartHigh,

    StartHigh(Stop) => Idle,
    High(Stop) => StartLow [Stop],
    StartLow(Stop) => Idle,
    Low(Stop) => Idle,
}

pub struct A49xx<S, D>
where
    S: OutputPin,
    D: OutputPin, {
    pub timings: StepperTimings,
    step_delay: Option<MicrosDurationU32>,

    step: S,
    dir: D,

    state_machine: StateMachine<StepperState>,
    step_state_machine: StateMachine<StepState>,

    spawn_fn: SpawnFn
}

impl<S, D> A49xx<S, D>
where
    S: OutputPin,
    D: OutputPin, {
    pub fn new(step: S, dir: D, spawn_fn: SpawnFn) -> Self {
        Self {
            timings: Default::default(),

            step_delay: None,
            step, dir,

            state_machine: StateMachine::<StepperState>::new(),
            step_state_machine: StateMachine::<StepState>::new(),

            spawn_fn
        }
    }

    pub fn set_speed(&mut self, speed: HertzU32) {
        self.step_delay = Some(speed.into_duration());

        if *self.state_machine.state() == StepperStateState::Idle {
            self.state_machine.consume(&StepperStateInput::Start).unwrap();
            (self.spawn_fn)();
        }
    }

    pub fn stop(&mut self) {
        self.state_machine.consume(&StepperStateInput::Stop).unwrap();
    }

    pub fn update(&mut self) -> Option<MicrosDurationU32> {
        if self.step_delay.is_none() {
            return None;
        }

        // TODO: control flow is fucked up
        match *self.state_machine.state() {
            StepperStateState::Idle => { None },
            StepperStateState::StartStepping => {
                self.step_state_machine.consume(&StepStateInput::Start).unwrap();
                self.state_machine.consume(&StepperStateInput::Started).unwrap();
                self.update()
            },
            StepperStateState::Stepping => {
                let state = match *self.step_state_machine.state() {
                    StepStateState::Idle => { StepperStateInput::Stopped },
                    StepStateState::StartHigh => {
                        self.step.set_high().ok();
                        self.step_state_machine.consume(&StepStateInput::PulseStart).unwrap();

                        return Some(self.timings.pulse_width);
                    },
                    StepStateState::StartLow => {
                        self.step.set_low().ok();
                        self.step_state_machine.consume(&StepStateInput::PulseStart).unwrap();

                        return Some(self.step_delay.unwrap());
                    },
                    StepStateState::High | StepStateState::Low => {
                        self.step_state_machine.consume(&StepStateInput::PulseEnd).unwrap();

                        return self.update();
                    },
                };
                self.state_machine.consume(&state).unwrap();
                self.update()
            },
            StepperStateState::StopStepping => {
                self.step_state_machine.consume(&StepStateInput::Stop).unwrap();
                if self.step_state_machine.state() == &StepStateState::Idle {
                    self.state_machine.consume(&StepperStateInput::Stopped).unwrap();
                }
                self.update()
            }
        }
    }
}

