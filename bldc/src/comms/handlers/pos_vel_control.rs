use super::HandlesMessage;
use crate::comms::fdcan::FdcanMessage;

use crate::commutation::pos_vel_control::PosVelControl;
use crate::commutation::Commutator;

pub struct Cmd {}

impl From<FdcanMessage> for Cmd {
    fn from(_message: FdcanMessage) -> Self {
        Cmd {}
    }
}
pub struct EnterPosVelControl {}

impl HandlesMessage<Cmd> for EnterPosVelControl {
    fn handle(&self, _cmd: Cmd) {
        //
        Commutator::set(PosVelControl::new().into());
    }
}
