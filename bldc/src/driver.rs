use cortex_m::peripheral as cm;
use stm32g4::stm32g474 as device;

use crate::util::stm32::{clock_setup, clocks::G4_CLOCK_SETUP, disable_dead_battery_pd};

pub struct Controller<S> {
    rcc: device::RCC,
    flash: device::FLASH,
    pwr: device::PWR,

    mode_state: S,
}

pub struct Init {}

fn init(
    nvic: device::NVIC,
    rcc: device::RCC,
    flash: device::FLASH,
    pwr: device::PWR,
) -> Controller<Init> {
    disable_dead_battery_pd(&pwr);

    // Set up the core, AHB, and peripheral buses.
    clock_setup(&pwr, &rcc, &flash, &G4_CLOCK_SETUP);

    // Enable Flash cache and prefetching for full speed.
    flash
        .acr
        .modify(|_, w| w.dcen().enabled().icen().enabled().prften().enabled());

    // TODO(blakely): Move this somewhere more appropriate?
    // Enable the FDCAN clock
    rcc.apb1enr1.modify(|_, w| w.fdcanen().set_bit());

    Controller {
        rcc,
        flash,
        pwr,
        mode_state: Init {},
    }
}

pub fn take_hardware() -> Controller<Init> {
    let cp = cm::Peripherals::take().unwrap();
    let p = device::Peripherals::take().unwrap();
    init(cp.NVIC, p.RCC, p.FLASH, p.PWR)
}
