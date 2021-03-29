#![no_std]
#![no_main]

// use cortex_m::iprintln;
// use cortex_m::{peripheral::syst, peripheral::ITM, Peripherals};
use cortex_m_rt::entry;

use bldc::comms::fdcan::Sram;
use bldc::driver;

#[cfg(feature = "panic-itm")]
use panic_itm as _;
// #[cfg(feature = "panic-halt")]
// use panic_halt as _; // you can put a breakpoint on `rust_begin_unwind` to
// catch panics

// TODO(blakely): Comment on all the stuff happens before we actually get
// here...
#[entry]
fn main() -> ! {
    // let cortex_peripherals = Peripherals::take().unwrap();
    // let mut g4 = device::Peripherals::take().unwrap();

    // stm32::clock_setup(
    //     &mut g4.PWR,
    //     &mut g4.RCC,
    //     &mut g4.FLASH,
    //     &stm32::clocks::G4_CLOCK_SETUP,
    // );

    let _foo = driver::take_hardware();

    // let fdcan = Fdcan::new();
    // let asdf = fdcan.extended_filters.filter(0);
    // asdf.set(
    //     ExtendedFilterMode::StoreRxFIFO0,
    //     ExtendedFilterType::Dual,
    //     0x3,
    //     0x7,
    // );

    let mut test = &Sram::take();

    let asdf = &test.standard_filters[0];
    let foo = asdf.read().sft();

    // let mut systick = cortex_peripherals.SYST;

    // systick.set_clock_source(syst::SystClkSource::Core);
    // systick.set_reload(170000);
    // systick.clear_current();
    // systick.enable_counter();

    // let itm = unsafe { &mut *ITM::ptr() };
    // let stim = &mut itm.stim[0];

    // loop {
    //     let mut toot = 0;
    //     while toot < 1000 {
    //         while !systick.has_wrapped() {
    //             // loop until it's wrapped
    //         }
    //         toot += 1;
    //     }
    //     iprintln!(stim, "Second tick");
    // }

    loop {}
}
