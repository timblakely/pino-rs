use crate::{comms::fdcan::FdcanMessage, foc::DQCurrents};

use super::HandlesMessage;

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
pub struct TorqueControl {}

impl HandlesMessage<Cmd> for TorqueControl {
    fn handle(&self, _cmd: Cmd) {
        //
    }
}
