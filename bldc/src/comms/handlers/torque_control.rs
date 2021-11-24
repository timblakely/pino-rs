use crate::{
    comms::{
        fdcan::FdcanMessage,
        messages::{FdcanID, MessageID},
    },
    control_loops::torque_control::TorqueControl,
    driver::{Driver, Ready},
    foc::DQCurrents,
};

use super::HandlesMessage;
use crate::control_loops::Controller;

pub struct Cmd {
    pub duration: f32,
    pub currents: DQCurrents,
}

impl From<FdcanMessage> for Cmd {
    fn from(message: FdcanMessage) -> Self {
        let buffer = message.data;
        Cmd {
            duration: f32::from_bits(buffer[0]),
            currents: DQCurrents {
                q: f32::from_bits(buffer[1]),
                d: f32::from_bits(buffer[2]),
            },
        }
    }
}

pub struct EnterTorqueControl {}

impl EnterTorqueControl {
    pub fn new() -> Self {
        EnterTorqueControl {}
    }
}

impl HandlesMessage<Cmd> for EnterTorqueControl {
    fn handle(&self, _driver: &mut Driver<Ready>, cmd: Cmd) {
        Controller::set(TorqueControl::new(cmd.duration, cmd.currents));
    }
}

impl FdcanID for EnterTorqueControl {
    const ID: MessageID = MessageID::EnterTorqueControl;
}
