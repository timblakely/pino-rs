use crate::block_while;
use cortex_m::peripheral as cm;
use stm32g4::stm32g474 as device;

use crate::util::stm32::{clock_setup, clocks::G4_CLOCK_SETUP, disable_dead_battery_pd};
use crate::{
    block_until,
    comms::fdcan::{self, Sram, FDCANSHARE},
};
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
    pub spi1: device::SPI1,
    pub spi3: device::SPI3,
    pub tim3: device::TIM3,
    pub dma1: device::DMA1,
    pub dmamux: device::DMAMUX,
}

pub struct Ready {
    pub spi1: device::SPI1,
}

pub fn take_hardware() -> Controller<Init> {
    let cp = cm::Peripherals::take().unwrap();
    let p = device::Peripherals::take().unwrap();

    init(
        cp.NVIC, p.RCC, p.FLASH, p.PWR, p.GPIOA, p.GPIOB, p.GPIOC, p.FDCAN1, p.SPI1, p.SPI3,
        p.TIM3, p.DMA1, p.DMAMUX,
    )
}

fn init(
    mut nvic: cm::NVIC,
    rcc: device::RCC,
    flash: device::FLASH,
    pwr: device::PWR,
    gpioa: device::GPIOA,
    gpiob: device::GPIOB,
    gpioc: device::GPIOC,
    fdcan: device::FDCAN1,
    spi1: device::SPI1,
    spi3: device::SPI3,
    tim3: device::TIM3,
    dma1: device::DMA1,
    dmamux: device::DMAMUX,
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

    // Enable various peripheral clocks
    rcc.ccipr.modify(|_, w| w.fdcansel().pllq());
    rcc.ahb1enr
        .modify(|_, w| w.dma1en().set_bit().dmamuxen().set_bit());
    rcc.apb1enr1
        .modify(|_, w| w.fdcanen().set_bit().tim3en().set_bit());
    rcc.apb2enr.modify(|_, w| w.spi1en().enabled());

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
            spi1,
            spi3,
            tim3,
            dma1,
            dmamux,
        },
    }
}

pub static mut TOOT: u16 = 1;
pub static mut MA702_REQUEST_ANGLE: u16 = 0;

impl Controller<Init> {
    // TODO(blakely): Move into a device-specific, feature-guarded trait
    fn configure_gpio(&self) {
        // Configure GPIOA pins
        // PA4 - SPI1 - ENC_CS - AF5
        // PA5 - SPI1 - ENC_SCK - AF5
        // PA6 - SPI1 - ENC_MISO - AF5
        // PA7 - SPI1 - ENC_MOSI - AF5
        // PA11 - FDCAN_RX, PUSHPULL, NOPULL, VERY_HIGH
        // PA12 - FDCAN_TX, PUSHPULL, NOPULL, VERY_HIGH
        let gpioa = &self.mode_state.gpioa;

        // Pin modes
        gpioa.moder.modify(|_, w| {
            w.moder4()
                .alternate()
                .moder5()
                .alternate()
                .moder6()
                .alternate()
                .moder7()
                .alternate()
                .moder11()
                .alternate()
                .moder12()
                .alternate()
        });

        // Alternate function settings
        gpioa
            .afrl
            .modify(|_, w| w.afrl4().af5().afrl5().af5().afrl6().af5().afrl7().af5());
        gpioa.afrh.modify(|_, w| w.afrh11().af9().afrh12().af9());

        // Output types
        gpioa.otyper.modify(|_, w| {
            w.ot4()
                .push_pull()
                .ot5()
                .push_pull()
                .ot6()
                .push_pull()
                .ot7()
                .push_pull()
                .ot11()
                .push_pull()
                .ot12()
                .push_pull()
        });

        // Speed
        gpioa.ospeedr.modify(|_, w| {
            w.ospeedr4()
                .very_high_speed()
                .ospeedr5()
                .very_high_speed()
                .ospeedr6()
                .very_high_speed()
                .ospeedr7()
                .very_high_speed()
                .ospeedr11()
                .very_high_speed()
                .ospeedr12()
                .very_high_speed()
        });

        // Pullup/down/float
        gpioa.pupdr.modify(|_, w| {
            w.pupdr4()
                .floating()
                .pupdr5()
                .floating()
                .pupdr6()
                .floating()
                .pupdr7()
                .floating()
                .pupdr11()
                .floating()
                .pupdr12()
                .floating()
        });
    }

    fn configure_timers(&self) {
        // Configure TIM3 for 1kHz polling of SPI1
        let tim3 = &self.mode_state.tim3;
        // Stop the timer if it's running for somet reason.
        tim3.cr1.modify(|_, w| w.cen().clear_bit());
        block_until!(tim3.cr1.read().cen().bit_is_clear());
        // Edge aligned mode, and up counting.
        tim3.cr1.modify(|_, w| w.dir().up().cms().edge_aligned());
        // Fire off a DMA on update (i.e. counter overflow)
        tim3.dier.modify(|_, w| w.ude().set_bit());
        // Assuming 170MHz core clock, set prescalar to 3 and ARR to 42500 for 170e6/42500/4=1kHz.
        // Why 3 and not 4? The timer cock is set to `core_clk / (PSC[PSC] + 1)`. If it were to use
        // the value directly it'd divide the clock by zero on reset, which would be A Bad Thing.
        // Safety: Upstream: This should have a proper range of 0-65535 in stm32-rs. 3 is well
        // within range.
        tim3.psc.write(|w| unsafe { w.psc().bits(3) });
        // Safety: Upstream: This should have a proper range of 0-65535 in stm32-rs. 42500 is within
        // range.
        tim3.arr.write(|w| unsafe { w.arr().bits(42500) });
    }

    fn configure_dma(&self) {
        // Configure DMA1 stream 1 to transfer a `0` into `SPI1[DR]` to trigger an SPI transaction,
        // off the update event from tim3.
        let dma = &self.mode_state.dma1;
        // Disable DMA channel if it's enabled.
        dma.ccr1.modify(|_, w| w.en().clear_bit());
        block_until!(dma.ccr1.read().en().bit_is_clear());
        // Configure for memory-to-peripheral mode @ 16-bit. Don't change address for either memory
        // or peripheral.
        dma.ccr1.modify(|_, w| unsafe {
            // Safety: Upstream: This should be a 2-bit enum. 0b01 = 16-bit
            w.msize()
                .bits(0b01)
                // Safety: Upstream: This should be a 2-bit enum. 0b01 = 16-bit
                .psize()
                .bits(0b01)
                .minc()
                .clear_bit()
                .pinc()
                .clear_bit()
                .circ()
                .set_bit()
                .dir()
                .set_bit()
        });
        // Just transfer a single value
        // Safety: Upstream: This should have a proper range of 0-65535 in stm32-rs.
        dma.cndtr1.write(|w| unsafe { w.ndt().bits(1) });
        // Target memory location
        {
            // Safety: This is the source of the DMA stream. We've configured it for 16-bit
            // and the address we're taking is a `u16`
            dma.cmar1
                .write(|w| unsafe { w.ma().bits(((&MA702_REQUEST_ANGLE) as *const _) as u32) });
        }
        // Target peripheral location
        {
            let spi = &self.mode_state.spi1;
            // Safety: Erm... its not? XD We're asking the DMA to stream data to an arbitrary
            // address, which is in no way shape or form safe. We've set it up so that it's a `u16`
            // transfer from the static above to `SPI[DR]`. YOLO
            dma.cpar1
                .write(|w| unsafe { w.pa().bits(((&spi.dr) as *const _) as u32) });
        }

        // Now we wire up the DMA triggers to their respective streams
        let dmamux = &self.mode_state.dmamux;
        // Note: DMAMUX channels 0-7 connected to DMA1 channels 1-8, 8-15=DMA2 1-8
        // TIM3 Update to the DMA stream 1 - TIM3_UP = 65
        // Safety: Upstream: This should be an enum.
        // TODO(blakely): Add enum values to `stm32-rs`
        dmamux.c0cr.modify(|_, w| unsafe { w.dmareq_id().bits(65) });

        // Enable the DMA stream.
        dma.ccr1.modify(|_, w| w.en().set_bit());
    }

    pub fn configure_peripherals(self) -> Controller<Ready> {
        self.configure_gpio();
        self.configure_timers();
        self.configure_dma();

        // SPI config
        let spi1 = self.mode_state.spi1;
        // Disable SPI, if enabled.
        spi1.cr1.modify(|_, w| w.spe().clear_bit());
        block_until! { spi1.cr1.read().spe().bit_is_clear() }
        spi1.cr1.modify(|_, w| {
            w.cpha()
                .clear_bit()
                .cpol()
                .clear_bit()
                .mstr()
                .set_bit()
                .br()
                .div128()
                .crcen()
                .clear_bit()
        });
        // TODO(blakely): experiment with NSSP=1, since "In the case of a single data transfer, it
        // forces the NSS pin high level after the transfer." - R0440,p1784
        spi1.cr2.modify(|_, w| {
            w.ssoe()
                .enabled()
                .frf()
                .clear_bit()
                .ds()
                .sixteen_bit()
                .nssp()
                .set_bit()
        });

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

        // Enable the SPI port
        spi1.cr1.modify(|_, w| w.spe().set_bit());

        // Kick off tim3.
        self.mode_state.tim3.cr1.modify(|_, w| w.cen().set_bit());

        let new_self = Controller {
            mode_state: Ready { spi1 },
        };
        new_self
    }
}

pub struct FdcanShared {
    pub sram: Sram,
    pub fdcan: device::FDCAN1,
}
