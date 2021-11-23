use super::fdcan::FdcanMessage;

// TODO(blakely): move into FDCAN file
trait FdcanMessageHandler {
    fn handler(self, msg: FdcanMessage);
}

trait HandlesMessage<T>
where
    T: From<FdcanMessage>,
{
    fn handle(self, msg: T);
}

mod torque_control {
    use super::FdcanMessage;
    use super::HandlesMessage;
    use crate::foc::DQCurrents;

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
        fn handle(self, cmd: Cmd) {
            //
        }
    }
}

// This implements effectively the same thing as the `enum_dispatch` crate. However, it currently
// doesn't handle associated types, which means we'd have to fall back to generics, and generic
// specialization doesn't really work without associated types in Rust at the moment. So until
// `enum_dispatch` supports associated types, we roll our own here.
// DEPENDS: https://gitlab.com/antonok/enum_dispatch/-/issues/30
enum MessageHandler {
    TorqueControl(torque_control::TorqueControl),
}

impl FdcanMessageHandler for MessageHandler {
    fn handler(self, msg: FdcanMessage) {
        use MessageHandler::*;
        match self {
            TorqueControl(inner) => inner.handle(msg.into()),
        }
    }
}

impl From<torque_control::TorqueControl> for MessageHandler {
    fn from(inner: torque_control::TorqueControl) -> Self {
        MessageHandler::TorqueControl(inner)
    }
}
