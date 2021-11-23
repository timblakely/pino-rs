use crate::{
    comms::fdcan::FdcanMessage, commutation::torque_control::TorqueControl, foc::DQCurrents,
};

use super::HandlesMessage;
use crate::commutation::Commutator;

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
    pub const ID: u32 = 0x17;

    pub fn new() -> Self {
        EnterTorqueControl {}
    }
}

impl HandlesMessage<Cmd> for EnterTorqueControl {
    fn handle(&self, cmd: Cmd) {
        Commutator::set(TorqueControl::new(cmd.duration, cmd.currents).into());
    }
}
