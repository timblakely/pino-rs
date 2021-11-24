use crate::{
    comms::{
        fdcan::FdcanMessage,
        messages::{FdcanID, MessageID},
    },
    control_loops::pos_vel_control::{PosVelState, PositionVelocity},
    driver::{Driver, Ready},
};

use super::HandlesMessage;

pub struct Cmd {
    pub position: f32,
    pub velocity: f32,
    pub stiffness_gain: f32,
    pub damping_gain: f32,
    pub torque_constant: f32,
}

impl From<FdcanMessage> for Cmd {
    fn from(message: FdcanMessage) -> Self {
        let buffer = message.data;
        Cmd {
            position: f32::from_bits(buffer[0]),
            velocity: f32::from_bits(buffer[1]),
            stiffness_gain: f32::from_bits(buffer[2]),
            damping_gain: f32::from_bits(buffer[3]),
            torque_constant: f32::from_bits(buffer[4]),
        }
    }
}

impl Into<PosVelState> for Cmd {
    fn into(self) -> PosVelState {
        PosVelState {
            position: self.position,
            velocity: self.velocity,
            stiffness_gain: self.stiffness_gain,
            damping_gain: self.damping_gain,
            torque_constant: self.torque_constant,
        }
    }
}

pub struct SetPosVel {}

impl SetPosVel {
    pub fn new() -> Self {
        SetPosVel {}
    }
}

impl HandlesMessage<Cmd> for SetPosVel {
    fn handle(&self, _driver: &mut Driver<Ready>, cmd: Cmd) {
        PositionVelocity::command(cmd.into());
    }
}

impl FdcanID for SetPosVel {
    const ID: MessageID = MessageID::SetPosVel;
}
