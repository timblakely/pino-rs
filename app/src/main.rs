#![no_std]
#![no_main]

use cortex_m::{peripheral::syst, Peripherals};
use cortex_m_rt::entry;

use stm32g4::stm32g474 as device;

#[cfg(feature = "panic-itm")]
use panic_itm as _;
// #[cfg(feature = "panic-halt")]
// use panic_halt as _; // you can put a breakpoint on `rust_begin_unwind` to catch panics

#[entry]
fn main() -> ! {
    panic!("Hello panic!");
    let peripherals = Peripherals::take().unwrap();
    let mut systick = peripherals.SYST;

    systick.set_clock_source(syst::SystClkSource::Core);
    systick.set_reload(16_000_000);
    systick.clear_current();
    systick.enable_counter();

    while !systick.has_wrapped() {
        // loop until it's wrapped
    }

    let nvic = device::NVIC::ptr();

    loop {
        // your code goes here
    }
}
