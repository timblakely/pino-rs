#![no_std]
#![no_main]
#![feature(unboxed_closures, fn_traits)]

use bldc::{
    comms::messages::{self, ExtendedFdcanFrame, Messages},
    commutation::{CallbackCurrentSensor, Commutator},
    driver,
};

#[cfg(feature = "panic-halt")]
use panic_halt as _;
#[cfg(feature = "panic-itm")]
use panic_itm as _; // you can put a breakpoint on `rust_begin_unwind` to catch panics

// TODO(blakely): Comment on all the stuff that happens before we actually get
// here...
#[cortex_m_rt::entry]
fn main() -> ! {
    let driver = driver::take_hardware().configure_peripherals();
    driver.listen(|fdcan, message| {
        match Messages::unpack_fdcan(message) {
            Some(Messages::IdleCurrentSense(m)) => {
                let acc = CallbackCurrentSensor::new(m.duration, |w| {
                    fdcan.send_message(
                        messages::Currents {
                            phase_a: w.phase_a,
                            phase_b: w.phase_b,
                            phase_c: w.phase_c,
                        }
                        .pack(),
                    );
                });
                Commutator::set(acc);
            }
            _ => (),
        };
    });
    loop {}
}
