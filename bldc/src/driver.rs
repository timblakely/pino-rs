use cortex_m::peripheral as cm;
use stm32g4::stm32g474 as device;

use crate::comms::fdcan::{self, Sram, FDCANSHARE, FDCAN_RECEIVE_BUF};
use crate::util::stm32::{clock_setup, clocks::G4_CLOCK_SETUP, disable_dead_battery_pd};
use third_party::m4vga_rs::util::armv7m::{disable_irq, enable_irq};

pub struct Controller<S> {
    #[allow(dead_code)]
    pub mode_state: S,
}

pub struct Init {
    pub fdcan: device::FDCAN1,
    pub gpioa: device::GPIOA,
    pub gpiob: device::GPIOB,
    pub gpioc: device::GPIOC,
}

pub struct Ready {}

fn init(
    mut nvic: cm::NVIC,
    rcc: device::RCC,
    flash: device::FLASH,
    pwr: device::PWR,
    gpioa: device::GPIOA,
    gpiob: device::GPIOB,
    gpioc: device::GPIOC,
    fdcan: device::FDCAN1,
) -> Controller<Init> {
    disable_dead_battery_pd(&pwr);

    // Make sure we don't receive any interrupts before we're ready.
    disable_irq(device::Interrupt::FDCAN1_INTR0_IT);
    disable_irq(device::Interrupt::FDCAN1_INTR1_IT);

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

    // Configure interrupt priorities.
    // Safety: messing with interrupt priorities is inherently unsafe, but we disabled our device
    // interrupts above.
    unsafe {
        nvic.set_priority(device::Interrupt::FDCAN1_INTR0_IT, 0x10);
        nvic.set_priority(device::Interrupt::FDCAN1_INTR1_IT, 0x10);
    }

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

        // Configure GPIOA pins
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

        // Configure FDCAN
        let mut fdcan = fdcan::take(self.mode_state.fdcan)
            .enter_init()
            // TODO(blakely): clean up this API.
            .set_extended_filter(
                0,
                fdcan::extended_filter::ExtendedFilterMode::StoreRxFIFO0,
                fdcan::extended_filter::ExtendedFilterType::Classic,
                0x1,
                0xFFF_FFFF,
            )
            .configure_protocol()
            .configure_timing()
            .configure_interrupts()
            .fifo_mode()
            .start();
        fdcan.send_message();
        *FDCANSHARE.try_lock().unwrap() = Some(fdcan.donate());

        fdcan::init_receive_buf();

        // Tx IRQ
        enable_irq(device::Interrupt::FDCAN1_INTR0_IT);
        // Rx IRQ
        enable_irq(device::Interrupt::FDCAN1_INTR1_IT);

        let new_self = Controller {
            mode_state: Ready {},
        };
        new_self
    }
}

pub struct FdcanShared {
    pub sram: Sram,
    pub fdcan: device::FDCAN1,
}

pub fn take_hardware() -> Controller<Init> {
    let cp = cm::Peripherals::take().unwrap();
    let p = device::Peripherals::take().unwrap();

    init(
        cp.NVIC, p.RCC, p.FLASH, p.PWR, p.GPIOA, p.GPIOB, p.GPIOC, p.FDCAN1,
    )
}
