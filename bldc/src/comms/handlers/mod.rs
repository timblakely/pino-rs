pub mod disable_control_loop;
pub mod pos_vel_control;
pub mod set_pos_vel;
pub mod torque_control;

use crate::control_loops::Controller;

use super::fdcan::FdcanMessage;

use disable_control_loop::DisableControlLoop;
use pos_vel_control::EnterPosVelControl;
use set_pos_vel::SetPosVel;
use torque_control::EnterTorqueControl;

trait HandlesMessage<T>
where
    T: From<FdcanMessage>,
{
    fn handle(&self, controller: &mut Controller, msg: T);
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

        impl $n {
            pub fn process(&self, controller: &mut Controller, msg: FdcanMessage) {
                use $n::*;
                match self {
                    $( $x(inner) => inner.handle(controller, msg.into()), )*
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
    EnterTorqueControl,
    EnterPosVelControl,
    SetPosVel,
    DisableControlLoop,
});
