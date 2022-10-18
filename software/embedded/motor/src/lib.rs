#![no_std]

use compare::{Compare, natural};
use num_traits::{Zero, NumCast};

use core::{cmp::Ordering::{Less, Equal, Greater}, intrinsics::transmute};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RotationDirection {
    Clockwise,
    Counterclockwise,
    None
}

pub trait SetDirection {
    fn set_direction(&mut self, direction: RotationDirection);
}

pub trait SetSpeed {
    type Speed;

    fn set_speed(&mut self, speed: Self::Speed);
}

pub trait GetSpeed {
    type Speed;

    fn get_speed(&mut self) -> Self::Speed;
}

pub struct Motor<D, S>
where
    D: SetDirection,
    S: SetSpeed
{
    dir: D,
    speed: S,

    current_direction: RotationDirection
}

impl<D, S> Motor<D, S>
where
    D: SetDirection,
    S: SetSpeed,
{
    pub fn new(direction_controller: D, speed_controller: S) -> Self {
        Self { 
            dir: direction_controller,
            speed: speed_controller,

            current_direction: RotationDirection::None
        }
    }
}

impl<D, S> SetSpeed for Motor<D, S>
where
    D: SetDirection,
    S: SetSpeed,
    S::Speed: Copy + Ord
{
    type Speed = i8;

    fn set_speed(&mut self, speed: Self::Speed) {
        let cmp = natural();
        let direction = match cmp.compare(&speed, &Self::Speed::zero()) {
            Less => { RotationDirection::Counterclockwise },
            Equal => { RotationDirection::None },
            Greater => { RotationDirection::Clockwise }
        };
        self.current_direction = direction;
        self.dir.set_direction(direction);

        let speed = unsafe { *transmute::<&i8, &S::Speed>(&speed.abs()) };

        self.speed.set_speed(speed);
    }
}

impl<D, S> GetSpeed for Motor<D, S>
where
    D: SetDirection,
    S: SetSpeed + GetSpeed,
    <S as GetSpeed>::Speed: NumCast
{
    type Speed = i8;

    fn get_speed(&mut self) -> Self::Speed {
        let speed = self.speed.get_speed();
        let speed: Self::Speed = NumCast::from(speed).unwrap();

        let sign = match self.current_direction {
            RotationDirection::None => 0,
            RotationDirection::Clockwise => 1,
            RotationDirection::Counterclockwise => -1
        };

        speed * sign
    }
}

