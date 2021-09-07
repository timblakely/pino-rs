#![no_std]
#![no_main]
#![feature(unboxed_closures, fn_traits)]

use core::mem::MaybeUninit;
extern crate alloc;

use alloc::boxed::Box;

use bldc::{
    allocator::initialize_heap,
    comms::messages::Messages,
    commutation::{ControlParameters, IdleCurrentSensor},
    driver,
};

#[cfg(feature = "panic-halt")]
use panic_halt as _;
#[cfg(feature = "panic-itm")]
use panic_itm as _; // you can put a breakpoint on `rust_begin_unwind` to catch panics

// TODO(blakely): Implement emergency stop.
fn emergency_stop() {}

static mut HEAP: [MaybeUninit<u8>; 1 << 12] = [MaybeUninit::<u8>::uninit(); 1 << 12];

// TODO(blakely): Comment on all the stuff that happens before we actually get
// here...
#[cortex_m_rt::entry]
fn main() -> ! {
    unsafe {
        initialize_heap(&mut HEAP);
    }

    // let initial_state = ControlParameters {
    //     pwm_duty: 0f32,
    //     q: 0f32,
    //     d: 0f32,
    // };

    let driver = driver::take_hardware().configure_peripherals();

    // Allocate the ACS on the heap.

    driver.listen(|message| {
        match Messages::unpack_fdcan(message) {
            // Some(Messages::ForcePwm(msg)) => control_params.pwm_duty = msg.pwm_duty,
            // Some(Messages::EStop(_)) => emergency_stop(),
            // Some(Messages::SetCurrents(msg)) => {
            //     control_params.q = msg.q;
            //     control_params.d = msg.d;
            // }
            Some(Messages::IdleCurrentSense(m)) => {
                let acc = Box::new(IdleCurrentSensor::new(m.duration));
                driver::Commutator::set(acc);
            }
            _ => {}
        };
    });
    loop {}
}
