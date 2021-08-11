#![no_std]
#![no_main]
#![feature(unboxed_closures, fn_traits)]

use bldc::{
    comms::messages::{Debug, Messages},
    driver,
};
use stm32g4::stm32g474::{self as device, interrupt};
use third_party::m4vga_rs::util::armv7m::clear_pending_irq;

#[cfg(feature = "panic-halt")]
use panic_halt as _;
#[cfg(feature = "panic-itm")]
use panic_itm as _; // you can put a breakpoint on `rust_begin_unwind` to catch panics

// TODO(blakely): Move somewhere sane
#[derive(Clone, Copy)]
struct PwmState {
    pwm_duty: f32,
}

fn test(debug: Debug, state: &mut PwmState) {
    state.pwm_duty = debug.bar;
}

// TODO(blakely): Comment on all the stuff that happens before we actually get
// here...
#[cortex_m_rt::entry]
fn main() -> ! {
    let initial_state = PwmState { pwm_duty: 0f32 };

    let controller = driver::take_hardware().configure_peripherals();

    controller.run(
        initial_state,
        |message, state| {
            match Messages::unpack_fdcan(message) {
                Some(Messages::Debug(x)) => test(x, state),
                _ => {}
            };
        },
        |_board| {
            let mut _asdf = 0;
            _asdf += 1;
        },
    );
    loop {}
}

#[interrupt]
fn FDCAN1_INTR0_IT() {
    bldc::comms::fdcan::fdcan1_tx_isr();
    clear_pending_irq(device::Interrupt::FDCAN1_INTR0_IT);
}

#[interrupt]
fn FDCAN1_INTR1_IT() {
    bldc::comms::fdcan::fdcan1_rx_isr();
    clear_pending_irq(device::Interrupt::FDCAN1_INTR1_IT);
}
