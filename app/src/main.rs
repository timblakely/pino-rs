#![no_std]
#![no_main]

use core::borrow::BorrowMut;

use cortex_m::iprintln;
use cortex_m::{peripheral::syst, peripheral::ITM, Peripherals};
use cortex_m_rt::entry;

use stm32g4::stm32g474 as device;

#[cfg(feature = "panic-itm")]
use panic_itm as _;
// #[cfg(feature = "panic-halt")]
// use panic_halt as _; // you can put a breakpoint on `rust_begin_unwind` to catch panics

// Disable dead battery pull down first thing. Almost certainly not necessary
// since it doesn't seem to do anything without the USBDEN bit set in
// RCC.APB1EN, but everything from STM32CubeMX does it so why not?
fn disable_dead_battery_pd(pwr: &mut device::PWR) {
    pwr.cr3.modify(|_, w| w.ucpd1_dbdis().bit(true));
}

// The g4 starts up in 16MHz using the internal oscillator. This function goes
// through the process of enabling boost voltasges, enabling the high-speed
// external clock domain, disabling the high-speed internal, and finally
// configuring the PLL, stepping up clock speeds in two stages to avoid locking
// the AHB bus by switching over to a clock signal that's too fast to sync to.
fn clock_setup(pwr: &mut device::PWR, rcc: &mut device::RCC) {
    // Enable the PWR and SYSCFG domains.
    rcc.apb1enr1.modify(|_, w| w.pwren().set_bit());
    rcc.apb2enr.modify(|_, w| w.syscfgen().set_bit());

    // Ensure that we're in the appropriate boost mode, bumping the core voltage
    // to 80mV to support high frequency operation.
    pwr.cr5.modify(|_, w| w.r1mode().set_bit());

    // Configure the high-speed external oscillator, and wait till the RCC
    // subsystem has stabilized.
    rcc.cr.modify(|_, w| w.hseon().set_bit());
    while rcc.cr.read().hserdy().bit_is_clear() {
        // One of the nice things about Rust is that while this code does use
        // `unsafe` under the hood - once you're in assembly land you can do all
        // sorts of terrible things to memory essentially at will - the API it
        // exposes _isn't_ marked `unsafe`. It's a nice way for crate and API
        // maintainers to let both users and rustc know that the code itself is as
        // safe as it can be.
        cortex_m::asm::nop();
    }

    // Disable the high-speed internal oscillator. No need to waste power when
    // the external is working!
    rcc.cr.modify(|_, w| w.hsion().clear_bit());
}

// TODO(blakely): Comment on all the stuff happens before we actually get
// here...
#[entry]
fn main() -> ! {
    let cortex_peripherals = Peripherals::take().unwrap();
    let mut g4 = device::Peripherals::take().unwrap();

    disable_dead_battery_pd(&mut g4.PWR);

    let mut systick = cortex_peripherals.SYST;

    systick.set_clock_source(syst::SystClkSource::Core);
    systick.set_reload(16_000_000);
    systick.clear_current();
    systick.enable_counter();

    let itm = unsafe { &mut *ITM::ptr() };
    let stim = &mut itm.stim[0];

    iprintln!(stim, "Hello now");

    while !systick.has_wrapped() {
        // loop until it's wrapped
    }

    loop {
        // your code goes here
    }
}
