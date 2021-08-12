#![no_std]
#![no_main]
#![feature(unboxed_closures, fn_traits)]

use bldc::{comms::messages::Messages, commutation::ControlParameters, driver};

#[cfg(feature = "panic-halt")]
use panic_halt as _;
#[cfg(feature = "panic-itm")]
use panic_itm as _; // you can put a breakpoint on `rust_begin_unwind` to catch panics

fn emergency_stop() {}

// TODO(blakely): Comment on all the stuff that happens before we actually get
// here...
#[cortex_m_rt::entry]
fn main() -> ! {
    let initial_state = ControlParameters {
        pwm_duty: 0f32,
        q: 0f32,
        d: 0f32,
    };

    let controller = driver::take_hardware().configure_peripherals();

    controller.run(
        initial_state,
        |message, control_params| {
            match Messages::unpack_fdcan(message) {
                Some(Messages::ForcePwm(msg)) => control_params.pwm_duty = msg.pwm_duty,
                Some(Messages::EStop(_)) => emergency_stop(),
                Some(Messages::SetCurrents(msg)) => {
                    control_params.q = msg.q;
                    control_params.d = msg.d;
                }
                _ => {}
            };
        },
        // Control loop
        |_control_params| {
            // This is called at 40kHz, and where any commutation happens.
        },
    );
    loop {}
}
