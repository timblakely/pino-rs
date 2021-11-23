pub mod pos_vel_control;
pub mod torque_control;

use super::fdcan::{FdcanMessage, FdcanMessageHandler};

use pos_vel_control::EnterPosVelControl;
use torque_control::TorqueControl;

trait HandlesMessage<T>
where
    T: From<FdcanMessage>,
{
    fn handle(&self, msg: T);
}

// This implements effectively the same thing as the `enum_dispatch` crate. However, it currently
// doesn't handle associated types, which means we'd have to fall back to generics, and generic
// specialization doesn't really work without associated types in Rust at the moment. So until
// `enum_dispatch` supports associated types, we roll our own here.
// DEPENDS: https://gitlab.com/antonok/enum_dispatch/-/issues/30
pub enum MessageHandler {
    TorqueControl(TorqueControl),
    EnterPosVelControl(EnterPosVelControl),
}

impl FdcanMessageHandler for MessageHandler {
    fn process(&self, msg: FdcanMessage) {
        use MessageHandler::*;
        match self {
            TorqueControl(inner) => inner.handle(msg.into()),
            EnterPosVelControl(inner) => inner.handle(msg.into()),
        }
    }
}

impl From<TorqueControl> for MessageHandler {
    fn from(inner: TorqueControl) -> Self {
        MessageHandler::TorqueControl(inner)
    }
}

impl From<EnterPosVelControl> for MessageHandler {
    fn from(inner: EnterPosVelControl) -> Self {
        MessageHandler::EnterPosVelControl(inner)
    }
}
