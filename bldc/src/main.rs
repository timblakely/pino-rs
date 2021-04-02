#![no_std]
#![no_main]

use bldc::comms::fdcan::extended_filter::{ExtendedFilterMode, ExtendedFilterType};
use bldc::comms::fdcan::Fdcan;
use bldc::driver;

#[cfg(feature = "panic-halt")]
use panic_halt as _;
#[cfg(feature = "panic-itm")]
use panic_itm as _; // you can put a breakpoint on `rust_begin_unwind` to catch panics

// TODO(blakely): Comment on all the stuff happens before we actually get
// here...
#[cortex_m_rt::entry]
fn main() -> ! {
    let foo = driver::take_hardware();

    // foo.gpioa.moder.modify(|_, w| w.moder5().output());
    // foo.gpioa.pupdr.modify(|_, w| w.pupdr5().floating());
    // foo.gpioa.otyper.modify(|_, w| w.ot5().clear_bit());
    // foo.gpioa
    //     .ospeedr
    //     .modify(|_, w| w.ospeedr5().very_high_speed());

    // foo.gpioa.bsrr.write(|w| w.bs5().set_bit());

    // let mut systick = foo.syst;

    // systick.set_clock_source(cortex_m::peripheral::syst::SystClkSource::Core);
    // systick.set_reload(170000);
    // systick.clear_current();
    // systick.enable_counter();
    // loop {
    //     let mut toot = 0;
    //     while toot < 500 {
    //         while !systick.has_wrapped() {
    //             // loop until it's wrapped
    //         }
    //         toot += 1;
    //     }
    //     foo.gpioa.bsrr.write(|w| w.bs5().set_bit());
    //     while toot < 1000 {
    //         while !systick.has_wrapped() {
    //             // loop until it's wrapped
    //         }
    //         toot += 1;
    //     }
    //     foo.gpioa.bsrr.write(|w| w.br5().set_bit());
    //     // iprintln!(stim, "Second tick");
    // }

    loop {}
}
