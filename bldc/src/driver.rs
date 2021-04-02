use cortex_m::peripheral as cm;
use stm32g4::stm32g474 as device;

use crate::util::stm32::{clock_setup, clocks::G4_CLOCK_SETUP, disable_dead_battery_pd};

pub struct Controller<S> {
    #[allow(dead_code)]
    rcc: device::RCC,

    #[allow(dead_code)]
    mode_state: S,
}

pub struct Init {
    pub gpioa: device::GPIOA,
    pub gpiob: device::GPIOB,
    pub gpioc: device::GPIOC,
}

pub struct Ready;

fn init(
    _nvic: cm::NVIC,
    rcc: device::RCC,
    flash: device::FLASH,
    pwr: device::PWR,
    gpioa: device::GPIOA,
    gpiob: device::GPIOB,
    gpioc: device::GPIOC,
) -> Controller<Init> {
    disable_dead_battery_pd(&pwr);

    // Set up the core, AHB, and peripheral buses.
    clock_setup(&pwr, &rcc, &flash, &G4_CLOCK_SETUP);

    // Enable Flash cache and prefetching for full speed.
    flash
        .acr
        .modify(|_, w| w.dcen().enabled().icen().enabled().prften().enabled());

    // Enable the FDCAN clock
    rcc.apb1enr1.modify(|_, w| w.fdcanen().set_bit());

    // Turn on GPIO clocks.
    rcc.ahb2enr.modify(|_, w| {
        w.gpioaen()
            .set_bit()
            .gpioben()
            .set_bit()
            .gpiocen()
            .set_bit()
    });

    // Turn on SPI1 (Encoder) clock.
    rcc.apb2enr.modify(|_, w| w.spi1en().set_bit());
    // Turn on SPI3 (DRV8323RS) clock.
    rcc.apb1enr1.modify(|_, w| w.spi3en().set_bit());

    Controller {
        rcc,
        mode_state: Init {
            gpioa,
            gpiob,
            gpioc,
        },
    }
}

impl Controller<Init> {
    pub fn configure_peripherals(mut self) -> Controller<Ready> {
        let new_self = Controller {
            rcc: self.rcc,
            mode_state: Ready {},
        };
        new_self
    }
}

pub fn take_hardware() -> Controller<Init> {
    let cp = cm::Peripherals::take().unwrap();
    let p = device::Peripherals::take().unwrap();
    // p.FDCAN1.xidam.write(|w| w.eidm().bits(12));
    // p.TIM1.arr.write(|w| w.bits(123));

    init(cp.NVIC, p.RCC, p.FLASH, p.PWR, p.GPIOA, p.GPIOB, p.GPIOC)
}
