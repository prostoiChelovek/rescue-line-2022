use bincode::{Decode, Encode};

#[derive(Clone, Copy, Encode, Decode, PartialEq, Debug)]
pub enum Command {
    Stop,
    SetSpeed(SetSpeedParams),
    OpenGripper,
    CloseGripper,
    LiftGripper,
    LowerGripper
}

#[derive(Clone, Copy, Encode, Decode, PartialEq, Debug)]
pub struct SetSpeedParams {
    pub left: i32,
    pub right: i32
}

