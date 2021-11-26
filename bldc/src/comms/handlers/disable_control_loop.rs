use crate::{
    comms::{
        fdcan::FdcanMessage,
        messages::{FdcanID, MessageID},
    },
    control_loops::torque_control::TorqueControl,
    foc::DQCurrents,
};

use super::HandlesMessage;
use crate::control_loops::Controller;

pub struct Cmd {}

impl From<FdcanMessage> for Cmd {
    fn from(_: FdcanMessage) -> Self {
        Cmd {}
    }
}

pub struct DisableControlLoop {}

impl DisableControlLoop {
    pub fn new() -> Self {
        DisableControlLoop {}
    }
}

impl HandlesMessage<Cmd> for DisableControlLoop {
    fn handle(&self, controller: &mut Controller, _cmd: Cmd) {
        controller.disable_loop();
    }
}

impl FdcanID for DisableControlLoop {
    const ID: MessageID = MessageID::DisableControlLoop;
}
