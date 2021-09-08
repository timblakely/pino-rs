#![no_std]
#![no_main]
#![feature(unboxed_closures, fn_traits)]

use bldc::{
    comms::messages::{ExtendedFdcanFrame, Messages},
    commutation::{Commutator, IdleCurrentSensor},
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
    // Acquire the driver.
    let driver = driver::take_hardware().configure_peripherals();

    // Listen for any incoming FDCAN messages.
    driver.listen(|fdcan, message| {
        // We've received a message via the FDCAN.
        match Messages::unpack_fdcan(message) {
            Some(Messages::IdleCurrentSense(m)) => {
                Commutator::set(IdleCurrentSensor::new(m.duration, |measurement| {
                    fdcan.send_message(measurement.pack());
                }));
            }
            _ => (),
        };
    });
    loop {}
}
