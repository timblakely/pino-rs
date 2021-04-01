#![no_std]
#![no_main]

// use cortex_m::iprintln;
// use cortex_m::{peripheral::syst, peripheral::ITM, Peripherals};
use cortex_m_rt::entry;

use bldc::comms::fdcan::extended_filter::{ExtendedFilterMode, ExtendedFilterType};
use bldc::comms::fdcan::Fdcan;
use bldc::driver;
use stm32g4::stm32g474 as device;

#[cfg(feature = "panic-itm")]
use panic_itm as _;
// #[cfg(feature = "panic-halt")]
// use panic_halt as _; // you can put a breakpoint on `rust_begin_unwind` to
// catch panics

// TODO(blakely): Comment on all the stuff happens before we actually get
// here...
#[entry]
fn main() -> ! {
    let _foo = driver::take_hardware();

    let mut fdcan = Fdcan::new();
    let asdf = &mut fdcan.sram.extended_filters[1];

    fdcan.set_extended_filter(
        0,
        ExtendedFilterMode::StoreRxFIFO0,
        ExtendedFilterType::Dual,
        0x3,
        0x7,
    );

    loop {}
}
