#![no_std]
#![feature(trait_alias)]
#![feature(associated_type_defaults)]

pub trait Update {
    fn update(&mut self, time_delta_seconds: f32);
}

pub trait GetPosition {
    type Position = f32;

    fn get_position(&self) -> Self::Position;
}

pub trait GetVelocity {
    fn get_velocity(&self) -> f32;
}

pub trait Encoder = Update + GetPosition + GetVelocity;

