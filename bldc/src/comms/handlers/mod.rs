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
// doesn't handle associated types, which means we'd have to fall back to generics and generic
// specialization doesn't really work without associated types in Rust at the moment. So until
// `enum_dispatch` supports associated types, we roll our own here.
// DEPENDS: https://gitlab.com/antonok/enum_dispatch/-/issues/30
macro_rules! dispatchable_enum {
    ( $n: ident { $( $x: ident,)* }) => {
        pub enum $n {
            $(
                $x($x),
            )*
        }

        $( from_impl!($n { $x }); )*

        impl FdcanMessageHandler for $n {
            fn process(&self, msg: FdcanMessage) {
                use $n::*;
                match self {
                    $( $x(inner) => inner.handle(msg.into()), )*
                }
            }
        }
    };
    ( $n: ident { $( $x: ident ),* ,}) => {
        dispatchable_enum!($n { $( $x, )* });
    };
}

macro_rules! from_impl {
    ( $h:ident { $n:ident } ) => {
        impl From<$n> for $h {
            fn from(inner: $n) -> Self {
                $h::$n(inner)
            }
        }
    };
}

dispatchable_enum!(MessageHandler {
    TorqueControl,
    EnterPosVelControl,
});
