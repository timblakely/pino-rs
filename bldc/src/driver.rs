use cortex_m::peripheral as cm;
use stm32g4::stm32g474 as device;

use crate::comms::fdcan;
use crate::util::stm32::{clock_setup, clocks::G4_CLOCK_SETUP, disable_dead_battery_pd};
use third_party::m4vga_rs::util::armv7m::disable_irq;

pub struct Controller<S> {
    #[allow(dead_code)]
    mode_state: S,
}

pub struct Init {
    pub fdcan: device::FDCAN1,
    pub gpioa: device::GPIOA,
    pub gpiob: device::GPIOB,
    pub gpioc: device::GPIOC,
}

pub struct Ready {
    fdcan: fdcan::Fdcan<fdcan::Running>,
}

fn init(
    _nvic: cm::NVIC,
    rcc: device::RCC,
    flash: device::FLASH,
    pwr: device::PWR,
    gpioa: device::GPIOA,
    gpiob: device::GPIOB,
    gpioc: device::GPIOC,
    fdcan: device::FDCAN1,
) -> Controller<Init> {
    disable_dead_battery_pd(&pwr);

    // Set up the core, AHB, and peripheral buses.
    clock_setup(&pwr, &rcc, &flash, &G4_CLOCK_SETUP);

    // Enable Flash cache and prefetching for full speed.
    flash
        .acr
        .modify(|_, w| w.dcen().enabled().icen().enabled().prften().enabled());

    // FDCAN configuration
    // Turn on PLLQ so that we can use that for FDCAN
    // TODO(blakely): Is this necessary? Can't we just use the PCLK1? CubeMX seems to think so...
    rcc.pllcfgr
        .modify(|_, w| w.pllqen().set_bit().pllq().div2());
    rcc.ccipr.modify(|_, w| w.fdcansel().pllq());
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
        mode_state: Init {
            fdcan,
            gpioa,
            gpiob,
            gpioc,
        },
    }
}

impl Controller<Init> {
    pub fn configure_peripherals(self) -> Controller<Ready> {
        let gpioa = self.mode_state.gpioa;

        // Configure GPIO pins
        // PA11 - FDCAN_RX, PUSHPULL, NOPULL, VERY_HIGH
        // PA12 - FDCAN_TX, PUSHPULL, NOPULL, VERY_HIGH
        gpioa.moder.modify(|_, w| {
            w
                // FDCAN_RX
                .moder11()
                .alternate()
                // FDCAN_RX
                .moder12()
                .alternate()
        });
        gpioa.afrh.modify(|_, w| {
            w
                // FDCAN_RX
                .afrh11()
                .af9()
                // FDCAN_TX
                .afrh12()
                .af9()
        });
        gpioa
            .otyper
            .modify(|_, w| w.ot11().push_pull().ot12().push_pull());
        gpioa.ospeedr.modify(|_, w| {
            w.ospeedr11()
                .very_high_speed()
                .ospeedr12()
                .very_high_speed()
        });
        gpioa
            .pupdr
            .modify(|_, w| w.pupdr11().floating().pupdr12().floating());

        // Make sure we don't receive any incoming messages before we're ready.
        disable_irq(device::Interrupt::FDCAN1_INTR0_IT);
        // TODO(blakely): clean up this API.
        let mut fdcan1 = fdcan::take(self.mode_state.fdcan).enter_init();
        fdcan1
            .set_extended_filter(
                1,
                fdcan::extended_filter::ExtendedFilterMode::StoreRxFIFO0,
                fdcan::extended_filter::ExtendedFilterType::Dual,
                0x1,
                0x3,
            )
            .configure_protocol()
            .configure_timing();

        let new_self = Controller {
            mode_state: Ready {
                fdcan: fdcan1.start(),
            },
        };
        new_self
    }
}

pub fn take_hardware() -> Controller<Init> {
    let cp = cm::Peripherals::take().unwrap();
    let p = device::Peripherals::take().unwrap();

    init(
        cp.NVIC, p.RCC, p.FLASH, p.PWR, p.GPIOA, p.GPIOB, p.GPIOC, p.FDCAN1,
    )
}
