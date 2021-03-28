#![no_std]
#![no_main]

use cortex_m::iprintln;
use cortex_m::{peripheral::syst, peripheral::ITM, Peripherals};
use cortex_m_rt::entry;

use stm32g4::stm32g474 as device;

#[cfg(feature = "panic-itm")]
use panic_itm as _;
// #[cfg(feature = "panic-halt")]
// use panic_halt as _; // you can put a breakpoint on `rust_begin_unwind` to
// catch panics

// Shamelessly lifted from m4vga-rs
macro_rules! block_while {
    ($condition:expr) => {
        while $condition {}
    };
}
macro_rules! block_until {
    ($condition:expr) => {
        block_while!(!$condition)
    };
}

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
fn clock_setup(pwr: &mut device::PWR, rcc: &mut device::RCC, flash: &mut device::FLASH) {
    use device::{flash, rcc};
    // Enable the PWR and SYSCFG domains.
    rcc.apb1enr1.modify(|_, w| w.pwren().set_bit());
    rcc.apb2enr.modify(|_, w| w.syscfgen().set_bit());

    // Ensure that we're in the appropriate boost mode, bumping the core voltage
    // to 80mV to support high frequency operation.
    pwr.cr5.modify(|_, w| w.r1mode().clear_bit());

    // Configure the high-speed external oscillator, and wait till the RCC
    // subsystem has stabilized.
    rcc.cr.modify(|_, w| w.hseon().set_bit());
    block_while! { rcc.cr.read().hserdy().bit_is_clear() }

    // Disable the high-speed internal oscillator. No need to waste power when
    // the external is working!
    rcc.cr
        .modify(|_, w| w.hsion().variant(rcc::cr::HSION_A::OFF));

    // Now retarget the PLL so that it's reading from the HSE, and make sure the
    // clock dividers are set correctly before we re-enable it.
    rcc.cr.modify(|_, w| w.pllon().clear_bit());
    block_while! { rcc.cr.read().pllrdy().bit_is_set() }
    // Configure the PLL. This is scoped to allow the `use` statement to be
    // dropped at the end. This entire block compiles down to a single `LDR`
    // instruction.
    {
        use rcc::pllcfgr as v;
        rcc.pllcfgr.write(|w| {
            w.pllsrc()
                .variant(v::PLLSRC_A::HSE)
                .pllm()
                .variant(v::PLLM_A::DIV6)
                .plln()
                .variant(v::PLLN_A::DIV85)
                .pllr()
                .variant(v::PLLR_A::DIV2)
        });
    }
    // Bring the PLL back online
    rcc.cr.modify(|_, w| w.pllon().set_bit());
    block_until! { rcc.cr.read().pllrdy().bit_is_set() }
    // Finally, enable the PLLR domain that controls... basically everything.
    rcc.pllcfgr.modify(|_, w| w.pllren().set_bit());

    // Now we set the system clock source to HSE. However, we can't ramp
    // directly from the 16MHz HSI directly to 170MHz. Have to enter a few
    // intermediate states first:

    // 1) Set AHB prescalar div to 2 (=8Mhz)
    rcc.cfgr
        .modify(|_, w| w.hpre().variant(rcc::cfgr::HPRE_A::DIV2));

    // 2) Lower the peripheral clocks all the way down so they don't freak out
    //    during transition.
    rcc.cfgr.modify(|_, w| {
        w.ppre1()
            .variant(rcc::cfgr::PPRE1_A::DIV16)
            .ppre2()
            .variant(rcc::cfgr::PPRE2_A::DIV16)
    });

    // 3) Now make the jump to the 170MHz PLL (currently at 170/2=85MHz).
    rcc.cfgr.modify(|_, w| w.sw().variant(rcc::cfgr::SW_A::PLL));

    // 4) Wait for sysclksource to change. Can take a few cycles.
    block_until! { rcc.cfgr.read().sws().variant() == rcc::cfgr::SWS_A::PLL }

    // 5) Modify the number of flash wait states according to the new SysClock.
    //    For 170MHz operation, we require four wait states.
    //    See section 3.3.3 of the reference manual (RM0440) for specific
    //    frequency ranges and wait states.
    flash
        .acr
        .modify(|_, w| w.latency().variant(flash::acr::LATENCY_A::FOUR));

    // 6) Set AHB prescalar div to 1 (=170Mhz)
    rcc.cfgr
        .modify(|_, w| w.hpre().variant(rcc::cfgr::HPRE_A::DIV1));

    // 7) Configure peripheral clocks back to where they're supposed to be.
    rcc.cfgr.modify(|_, w| {
        w.ppre1()
            .variant(rcc::cfgr::PPRE1_A::DIV1)
            .ppre2()
            .variant(rcc::cfgr::PPRE2_A::DIV1)
    });

    // TODO(blakely): Configure individual peripheral clocks here too.
}

// TODO(blakely): Comment on all the stuff happens before we actually get
// here...
#[entry]
fn main() -> ! {
    let cortex_peripherals = Peripherals::take().unwrap();
    let mut g4 = device::Peripherals::take().unwrap();

    disable_dead_battery_pd(&mut g4.PWR);

    clock_setup(&mut g4.PWR, &mut g4.RCC, &mut g4.FLASH);

    let mut systick = cortex_peripherals.SYST;

    systick.set_clock_source(syst::SystClkSource::Core);
    systick.set_reload(170000);
    systick.clear_current();
    systick.enable_counter();

    let itm = unsafe { &mut *ITM::ptr() };
    let stim = &mut itm.stim[0];

    loop {
        let asdf = systick.csr.read();
        let mut toot = 0;
        while toot < 1000 {
            while !systick.has_wrapped() {
                // loop until it's wrapped
            }
            toot += 1;
        }
        iprintln!(stim, "Hello now");
    }
}
