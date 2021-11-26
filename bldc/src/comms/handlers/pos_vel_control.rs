use super::HandlesMessage;
use crate::comms::fdcan::FdcanMessage;

use crate::comms::messages::{FdcanID, MessageID};
use crate::control_loops::pos_vel_control::PositionVelocity;
use crate::control_loops::Controller;

pub struct Cmd {}

impl From<FdcanMessage> for Cmd {
    fn from(_message: FdcanMessage) -> Self {
        Cmd {}
    }
}

pub struct EnterPosVelControl {}

impl EnterPosVelControl {
    pub fn new() -> Self {
        EnterPosVelControl {}
    }
}

impl HandlesMessage<Cmd> for EnterPosVelControl {
    fn handle(&self, controller: &mut Controller, _cmd: Cmd) {
        controller.set_loop(PositionVelocity::new());
    }
}

impl FdcanID for EnterPosVelControl {
    const ID: MessageID = MessageID::EnterPosVelControl;
}
