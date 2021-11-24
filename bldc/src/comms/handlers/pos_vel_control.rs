use super::HandlesMessage;
use crate::comms::fdcan::FdcanMessage;

use crate::comms::messages::{FdcanID, MessageID};
use crate::control_loops::pos_vel_control::PositionVelocity;
use crate::control_loops::Commutator;

pub struct Cmd {}

impl From<FdcanMessage> for Cmd {
    fn from(_message: FdcanMessage) -> Self {
        Cmd {}
    }
}

pub struct EnterPosVelControl {}

impl EnterPosVelControl {
    pub const ID: u32 = 0x18;

    pub fn new() -> Self {
        EnterPosVelControl {}
    }
}

impl HandlesMessage<Cmd> for EnterPosVelControl {
    fn handle(&self, _cmd: Cmd) {
        Commutator::set(PositionVelocity::new());
    }
}

impl FdcanID for EnterPosVelControl {
    const ID: MessageID = MessageID::EnterPosVelControl;
}
