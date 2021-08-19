// Following the example of stm32-rs

use cortex_m::peripheral::{syst::SystClkSource, SYST};
use stm32g4::stm32g474 as device;
use third_party::m4vga_rs::util::{spin_lock::SpinLock, sync};

// Shamelessly lifted from m4vga-rs
#[macro_export]
macro_rules! block_while {
    ($condition:expr) => {
        while $condition {}
    };
}
#[macro_export]
macro_rules! block_until {
    ($condition:expr) => {
        block_while!(!$condition)
    };
}

// Representation of the configuration of the internal clocks on the G4
pub struct G4ClockConfig {
    pub crystal_hz: u32,                               // HSE crystal frequency
    pub crystal_divisor: device::rcc::pllcfgr::PLLM_A, // PLLM setting
    pub vco_multipler: device::rcc::pllcfgr::PLLN_A,   // PLLN vco PLL setting
    pub core_divisor: device::rcc::pllcfgr::PLLR_A,    // PLLR core divisor
    pub ahb_divisor: device::rcc::cfgr::HPRE_A,        // AHB prescalar
    pub apb1_divisor: device::rcc::cfgr::PPRE1_A,      // APB1 prescalar
    pub apb2_divisor: device::rcc::cfgr::PPRE2_A,      // APB2 prescalar
}

/// Configure the various clocks
///
/// The g4 starts up in 16MHz using the internal oscillator. This function goes
/// through the process of enabling boost voltasges, enabling the high-speed
/// external clock domain, disabling the high-speed internal, and finally
/// configuring the PLL, stepping up clock speeds in two stages to avoid locking
/// the AHB bus by switching over to a clock signal that's too fast to sync to.
pub fn clock_setup(
    pwr: &device::PWR,
    rcc: &device::RCC,
    flash: &device::FLASH,
    cfg: &G4ClockConfig,
) {
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
                .variant(cfg.crystal_divisor)
                .plln()
                .variant(cfg.vco_multipler)
                .pllr()
                .variant(cfg.core_divisor)
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

    // 1) Set AHB prescalar div to 2 (=8Mhz, since we started at 16MHz)
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
    rcc.cfgr.modify(|_, w| w.hpre().variant(cfg.ahb_divisor));

    // 7) Configure peripheral clocks back to where they're supposed to be.
    rcc.cfgr.modify(|_, w| {
        w.ppre1()
            .variant(cfg.apb1_divisor)
            .ppre2()
            .variant(cfg.apb2_divisor)
    });

    // TODO(blakely): Configure individual peripheral clocks here too.
}

pub mod clocks {
    use super::device::rcc::{cfgr, pllcfgr};
    use super::G4ClockConfig;
    pub static G4_CLOCK_SETUP: G4ClockConfig = G4ClockConfig {
        crystal_hz: 24_000_000,
        crystal_divisor: pllcfgr::PLLM_A::DIV6,
        vco_multipler: pllcfgr::PLLN_A::DIV85,
        core_divisor: pllcfgr::PLLR_A::DIV2,
        ahb_divisor: cfgr::HPRE_A::DIV1,
        apb1_divisor: cfgr::PPRE1_A::DIV1,
        apb2_divisor: cfgr::PPRE2_A::DIV1,
    };
}

// Disable dead battery pull down first thing. Almost certainly not necessary
// since it doesn't seem to do anything without the USBDEN bit set in
// RCC.APB1EN, but everything from STM32CubeMX does it so why not?
pub fn disable_dead_battery_pd(pwr: &device::PWR) {
    pwr.cr3.modify(|_, w| w.ucpd1_dbdis().bit(true));
}

static SYSTICK_SHARED: SpinLock<Option<SYST>> = SpinLock::new(None);

// Small wrapper to make sure that SYSTICK_SHARED doesn't escape this module.
pub fn donate_systick(mut systick: device::SYST) {
    *SYSTICK_SHARED.try_lock().unwrap() = Some(systick);
}

// Use SysTick as a blocking timer instead of sacrificing a Timer peripheral. Since it's blocking,
// we don't have to worry about the overhead of the interrupt, so we can actually do
// microsecond-level blocking with a spinguard. Note that this disables the SysTick interrupt.
// TODO(blakely): Add option to block interrupts as well.
pub fn blocking_sleep_us(us: u32) {
    let systick = &mut sync::acquire_hw(&SYSTICK_SHARED);
    systick.disable_interrupt();
    systick.disable_counter();
    systick.set_clock_source(SystClkSource::Core);
    // TODO(blakely): we're assuming a 170MHz core clock here. Actually pull the number from
    // somewhere.
    const TICKS_PER_US: u32 = (170e6 / 1e6) as u32;
    let desired_ticks = us * TICKS_PER_US;
    // The timer is 24 bit, so we have to loop if the desired number of ticks is > 2^24.
    let (mut loop_count, remainder, reload) = match desired_ticks >= 1 << 24 {
        true => (
            desired_ticks >> 24,
            desired_ticks & ((1 << 24) - 1),
            (1 << 24) - 1,
        ),
        false => (1, 0, desired_ticks),
    };
    systick.clear_current();
    systick.set_reload(reload);
    let _ = systick.has_wrapped();
    systick.enable_counter();
    while loop_count > 0 {
        while !systick.has_wrapped() {}
        loop_count -= 1;
    }
    if remainder > 0 {
        systick.set_reload(remainder);
        while !systick.has_wrapped() {}
    }
    systick.disable_counter();
}
