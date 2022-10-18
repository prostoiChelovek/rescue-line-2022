#![no_std]

#![feature(convert_float_to_int)]

use core::convert::FloatToInt;

use embedded_hal::{
    digital::v2::{OutputPin, PinState},
    PwmPin,
};

use motor::{RotationDirection, SetSpeed, GetSpeed, SetDirection};

pub struct PwmSetSpeed<P>
where
    P: PwmPin,
    P::Duty: From<u8>
{
    pin: P,
    pub min_speed: u8,

    current_speed: u8
}

impl<P> PwmSetSpeed<P>
where
    P: PwmPin,
    P::Duty: From<u8>
{
    pub fn new(pin: P, min_speed: u8) -> Self {
        let mut pin = pin;
        pin.enable();

        Self {
            pin, min_speed,
            current_speed: 0
        }
    }
}

impl<P: PwmPin> SetSpeed for PwmSetSpeed<P> 
where
    P: PwmPin,
    P::Duty: From<u8> + Into<f32>,
    f32: FloatToInt<P::Duty>
{
    type Speed = u8;

    fn set_speed(&mut self, speed: Self::Speed) {
        self.current_speed = speed.min(100);
        let speed = speed + self.min_speed;
        let speed = speed.max(0).min(100);
        let speed: f32 = speed.into();

        let max_duty: f32 = self.pin.get_max_duty().into();
        let duty = max_duty * speed / 100_f32;
        let duty = unsafe { duty.to_int_unchecked::<P::Duty>() };

        self.pin.set_duty(duty);
    }
}


impl<P: PwmPin> GetSpeed for PwmSetSpeed<P> 
where
    P: PwmPin,
    P::Duty: From<u8>,
{
    type Speed = u8;

    fn get_speed(&mut self) -> <Self as GetSpeed>::Speed {
        self.current_speed
    }
}

pub struct TwoPinSetDirection<F, B>
where
    F: OutputPin,
    B: OutputPin,
{
    fwd: F,
    back: B
}

impl<F, B> TwoPinSetDirection<F, B> 
where
    F: OutputPin,
    B: OutputPin,
{
    pub fn new(fwd_pin: F, back_pin: B) -> Self {
        TwoPinSetDirection {
            fwd: fwd_pin,
            back: back_pin,
        }
    }

    fn set_outs(&mut self, fwd: bool, back: bool) {
        self.fwd.set_state(PinState::from(fwd)).ok();
        self.back.set_state(PinState::from(back)).ok();
    }
}

impl<F, B> SetDirection for TwoPinSetDirection<F, B> 
where
    F: OutputPin,
    B: OutputPin,
{
    fn set_direction(&mut self, direction: RotationDirection) {
        match direction {
            RotationDirection::Clockwise => { self.set_outs(true, false) },
            RotationDirection::Counterclockwise => { self.set_outs(false, true) },
            RotationDirection::None => { self.set_outs(false, false) },
        }
    }
}

