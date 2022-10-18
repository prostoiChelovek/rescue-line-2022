#![no_std]

use core::{ops::Add, fmt::Display};

use num_traits::{NumCast, Signed, ToPrimitive, bounds::Bounded};

use pid::Pid;

use motor::{SetSpeed, GetSpeed};
use encoder::{Encoder, Update, GetPosition};

pub struct Wheel<S, E>
where
    S: SetSpeed + GetSpeed,
    E: Encoder,
    f32: From<<E as GetPosition>::Position>
{
    speed: S,
    encoder: E,

    pid: Pid<f32>,

    pub max_speed: f32,
    pub radius: f32
}

impl<S, E> Wheel<S, E>
where
    S: SetSpeed + GetSpeed,
    E: Encoder,
    f32: From<<E as GetPosition>::Position>
{
    pub fn new(speed_controller: S, encoder: E, pid: Pid<f32>, max_speed_rps: f32, radius_cm: f32) -> Self {
        let max_speed_cm = max_speed_rps * radius_cm;

        Self {
            speed: speed_controller,
            encoder,

            pid,

            max_speed: max_speed_cm,
            radius: radius_cm
        }
    }

    fn velocity_to_percent(&self, vel: f32) -> f32 {
        vel / self.max_speed * 100.0
    }

    fn to_cm(&self, val: f32) -> f32 {
        val * self.radius
    }

    pub fn get_target_speed(&self) -> f32 {
        self.max_speed * (self.pid.setpoint / 100.0)
    }
}

// TODO: it is kinda incorrect to use encoder's Update here, but it'll do for now
// TODO: clean up this fucking mess with traits
impl<S, E> Update for Wheel<S, E>
where
    S: SetSpeed + GetSpeed,
    <S as SetSpeed>::Speed: NumCast + Bounded,
    <S as GetSpeed>::Speed: NumCast + Add + Copy,
    <<S as GetSpeed>::Speed as Add>::Output: ToPrimitive + Display,
    E: Encoder,
    f32: From<<E as GetPosition>::Position>
{
    fn update(&mut self, time_delta_seconds: f32) {
        let min_speed_val: f32 = NumCast::from(<S as SetSpeed>::Speed::min_value()).unwrap();
        let max_speed_val: f32 = NumCast::from(<S as SetSpeed>::Speed::max_value()).unwrap();
        let min_speed_val = if min_speed_val.abs() > max_speed_val { -max_speed_val } else { min_speed_val };

        self.encoder.update(time_delta_seconds);

        let velocity = self.to_cm(self.encoder.get_velocity());
        let velocity = self.velocity_to_percent(velocity);

        let control = self.pid.next_control_output(velocity).output;
        let current_speed: f32 = NumCast::from(self.speed.get_speed()).unwrap();
        let new_speed = current_speed + control;
        let new_speed = new_speed.max(min_speed_val).min(max_speed_val);

        self.speed.set_speed(NumCast::from(new_speed).unwrap());
    }
}

impl<S, E> SetSpeed for Wheel<S, E>
where
    S: SetSpeed + GetSpeed,
    E: Encoder,
    f32: From<<E as GetPosition>::Position>
{
    type Speed = f32;

    fn set_speed(&mut self, speed: Self::Speed) {
        self.pid.setpoint = self.velocity_to_percent(speed);
    }
}

impl<S, E> GetSpeed for Wheel<S, E>
where
    S: SetSpeed + GetSpeed,
    E: Encoder,
    f32: From<<E as GetPosition>::Position>
{
    type Speed = f32;

    fn get_speed(&mut self) -> Self::Speed {
        self.to_cm(self.encoder.get_velocity())
    }
}

impl<S, E> GetPosition for Wheel<S, E>
where
    S: SetSpeed + GetSpeed,
    E: Encoder,
    f32: From<<E as GetPosition>::Position>
{
    fn get_position(&self) -> Self::Position {
        self.to_cm(self.encoder.get_position().into())
    }
}

