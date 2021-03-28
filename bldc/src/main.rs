#![no_std]
#![no_main]

use cortex_m::iprintln;
use cortex_m::{peripheral::syst, peripheral::ITM, Peripherals};
use cortex_m_rt::entry;

use stm32g4::stm32g474 as device;

use bldc::util::stm32;

#[cfg(feature = "panic-itm")]
use panic_itm as _;
// #[cfg(feature = "panic-halt")]
// use panic_halt as _; // you can put a breakpoint on `rust_begin_unwind` to
// catch panics

// Disable dead battery pull down first thing. Almost certainly not necessary
// since it doesn't seem to do anything without the USBDEN bit set in
// RCC.APB1EN, but everything from STM32CubeMX does it so why not?
fn disable_dead_battery_pd(pwr: &mut device::PWR) {
    pwr.cr3.modify(|_, w| w.ucpd1_dbdis().bit(true));
}

// TODO(blakely): Comment on all the stuff happens before we actually get
// here...
#[entry]
fn main() -> ! {
    let cortex_peripherals = Peripherals::take().unwrap();
    let mut g4 = device::Peripherals::take().unwrap();

    disable_dead_battery_pd(&mut g4.PWR);

    stm32::clock_setup(
        &mut g4.PWR,
        &mut g4.RCC,
        &mut g4.FLASH,
        &stm32::clocks::G4_CLOCK_SETUP,
    );

    let mut systick = cortex_peripherals.SYST;

    systick.set_clock_source(syst::SystClkSource::Core);
    systick.set_reload(170000);
    systick.clear_current();
    systick.enable_counter();

    let itm = unsafe { &mut *ITM::ptr() };
    let stim = &mut itm.stim[0];

    loop {
        let mut toot = 0;
        while toot < 1000 {
            while !systick.has_wrapped() {
                // loop until it's wrapped
            }
            toot += 1;
        }
        iprintln!(stim, "Second tick");
    }
}
