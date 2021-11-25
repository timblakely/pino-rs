use super::HandlesMessage;
use crate::comms::fdcan::FdcanMessage;

use crate::comms::messages::{FdcanID, MessageID};
use crate::control_loops::Controller;

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
    fn handle(&self, _controller: &mut Controller, _cmd: Cmd) {
        // Controller::set(PositionVelocity::new());
    }
}

impl FdcanID for EnterPosVelControl {
    const ID: MessageID = MessageID::EnterPosVelControl;
}
