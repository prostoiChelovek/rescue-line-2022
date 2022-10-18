#![no_std]

#![feature(trait_alias)]

use core::ops::{Add, Sub, Mul, Div};

use embedded_hal::{
    digital::v2::OutputPin,
    PwmPin,
};

use motor::{RotationDirection, SetSpeed, GetSpeed, SetDirection};

pub trait NumOps<T: Sized> = Add<T, Output = T> 
                                + Sub<T, Output = T> 
                                + Div<T, Output = T> 
                                + Mul<T, Output = T>;


fn map_range<T: Copy + NumOps<T>>(from_range: (T, T), to_range: (T, T), s: T) -> T 
{
    to_range.0 + (s - from_range.0) * (to_range.1 - to_range.0) / (from_range.1 - from_range.0)
}

pub struct TwoWirteDriver<PWM, DIR>
where
    PWM: PwmPin,
    PWM::Duty: From<u8> + Copy + NumOps<PWM::Duty>,
    DIR: OutputPin
{
    speed_pin: PWM,
    dir_pin: DIR,

    current_speed: u8,
    current_direction: RotationDirection,

    pub min_speed: u8,
}


impl<PWM, DIR> TwoWirteDriver<PWM, DIR>
where
    PWM: PwmPin,
    PWM::Duty: From<u8> + Copy + NumOps<PWM::Duty>,
    DIR: OutputPin
{
    pub fn new(mut pwm: PWM, dir: DIR, min_speed: u8) -> Self {
        pwm.enable();

        Self {
            speed_pin: pwm,
            dir_pin: dir,
            min_speed,
            current_speed: 0,
            current_direction: RotationDirection::Clockwise
        }
    }
}

impl<PWM, DIR> SetSpeed for TwoWirteDriver<PWM, DIR>
where
    PWM: PwmPin,
    PWM::Duty: From<u8> + Copy + NumOps<PWM::Duty>,
    DIR: OutputPin
{
    type Speed = u8;

    fn set_speed(&mut self, speed: Self::Speed) {
        self.current_speed = speed.min(100);
        let duty = {
            let duty: PWM::Duty = map_range(
                (1_u8.into(), 100_u8.into()),
                (self.min_speed.into(), self.speed_pin.get_max_duty()),
                self.current_speed.into()); 
            if self.current_direction == RotationDirection::Clockwise {
                self.speed_pin.get_max_duty() - duty
            }
            else {
                duty
            }
        };

        self.speed_pin.set_duty(duty);
    }
}


impl<PWM, DIR> GetSpeed for TwoWirteDriver<PWM, DIR>
where
    PWM: PwmPin,
    PWM::Duty: From<u8> + Copy + NumOps<PWM::Duty>,
    DIR: OutputPin
{
    type Speed = u8;

    fn get_speed(&mut self) -> <Self as GetSpeed>::Speed {
        self.current_speed
    }
}

impl<PWM, DIR> SetDirection for TwoWirteDriver<PWM, DIR>
where
    PWM: PwmPin,
    PWM::Duty: From<u8> + Copy + NumOps<PWM::Duty>,
    DIR: OutputPin
{
    fn set_direction(&mut self, direction: RotationDirection) {
        self.current_direction = direction;
        match direction {
            RotationDirection::Clockwise => { self.dir_pin.set_high().ok(); },
            RotationDirection::Counterclockwise => { self.dir_pin.set_low().ok(); },
            RotationDirection::None => { },
        };
        // the pwm value depends on direction
        self.set_speed(self.current_speed);
    }
}

