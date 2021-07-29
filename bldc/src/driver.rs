use crate::util::stm32::{clock_setup, clocks::G4_CLOCK_SETUP, disable_dead_battery_pd};
use crate::{
    block_until,
    comms::fdcan::{self, Sram, FDCANSHARE},
};
use crate::{
    block_while,
    ic::drv8323rs,
    ic::ma702::{self, Ma702, Streaming},
};
use cortex_m::peripheral as cm;
use drv8323rs::{Drv8323rs, DrvState};
use stm32g4::stm32g474 as device;
use third_party::m4vga_rs::util::armv7m::{disable_irq, enable_irq};

pub struct Controller<S> {
    pub mode_state: S,
}

pub struct Init {
    pub fdcan: device::FDCAN1,
    pub gpioa: device::GPIOA,
    pub gpiob: device::GPIOB,
    pub gpioc: device::GPIOC,
    pub spi1: device::SPI1,
    pub spi3: device::SPI3,
    pub tim1: device::TIM1,
    pub tim3: device::TIM3,
    pub dma1: device::DMA1,
    pub dmamux: device::DMAMUX,
    pub adc4: device::ADC4,
    pub syst: device::SYST,
}

pub struct Ready<D: DrvState> {
    pub ma702: Ma702<Streaming>,
    pub drv: Drv8323rs<D>,
    pub gpioa: device::GPIOA,
    // TODO(blakely): This shouldn't need to be public; implement wrapper function.
    pub syst: device::SYST,
}

pub fn take_hardware() -> Controller<Init> {
    let cp = cm::Peripherals::take().unwrap();
    let p = device::Peripherals::take().unwrap();

    init(
        cp.NVIC, p.RCC, p.FLASH, p.PWR, p.GPIOA, p.GPIOB, p.GPIOC, p.FDCAN1, p.SPI1, p.SPI3,
        p.TIM1, p.TIM3, p.DMA1, p.DMAMUX, p.ADC4, cp.SYST,
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
    tim1: device::TIM1,
    tim3: device::TIM3,
    dma1: device::DMA1,
    dmamux: device::DMAMUX,
    adc4: device::ADC4,
    syst: device::SYST,
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
        .modify(|_, w| w.dma1en().enabled().dmamuxen().enabled());
    rcc.ahb2enr
        .modify(|_, w| w.adc12en().enabled().adc345en().enabled());
    rcc.apb1enr1
        .modify(|_, w| w.fdcanen().enabled().tim3en().enabled());
    rcc.apb2enr
        .modify(|_, w| w.spi1en().enabled().tim1en().enabled());

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
            tim1,
            tim3,
            dma1,
            dmamux,
            adc4,
            syst,
        },
    }
}

impl Controller<Init> {
    // TODO(blakely): Move into a device-specific, feature-guarded trait
    fn configure_gpio(&self) {
        // TODO(blakely): Implement split-borrowing ro allow devices to take their own pins.
        // Configure GPIOA pins
        // PA4 - SPI1 - ENC_CS - AF5
        // PA5 - SPI1 - ENC_SCK - AF5
        // PA6 - SPI1 - ENC_MISO - AF5
        // PA7 - SPI1 - ENC_MOSI - AF5
        // PA7 - TIM1 - INH_A - AF6
        // PA8 - TIM1 - INH_B - AF6
        // PA9 - TIM1 - INH_C - AF6
        // PA11 - FDCAN_RX, PUSHPULL, NOPULL, VERY_HIGH
        // PA12 - FDCAN_TX, PUSHPULL, NOPULL, VERY_HIGH
        // PA15 - SPI3 - DRV_CS - AF6
        // PB5 - SPI3 - DRV_MOSI - AF6
        // PB9 - LED 1
        // PC6 - DRV_ENABLE
        // PC10 - SPI3 - DRV_SCK - AF6
        // PC11 - SPI3 - DRV_MISO - AF6
        let gpioa = &self.mode_state.gpioa;
        let gpiob = &self.mode_state.gpiob;
        let gpioc = &self.mode_state.gpioc;

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
                .moder8()
                .alternate()
                .moder9()
                .alternate()
                .moder10()
                .alternate()
                .moder11()
                .alternate()
                .moder12()
                .alternate()
                .moder15()
                .alternate()
        });
        gpiob
            .moder
            .modify(|_, w| w.moder5().alternate().moder9().output().moder12().analog());
        gpioc.moder.modify(|_, w| {
            w.moder6()
                .output()
                .moder10()
                .alternate()
                .moder11()
                .alternate()
        });

        // Alternate function settings
        gpioa
            .afrl
            .modify(|_, w| w.afrl4().af5().afrl5().af5().afrl6().af5().afrl7().af5());
        gpioa.afrh.modify(|_, w| {
            w.afrh8()
                .af6()
                .afrh9()
                .af6()
                .afrh10()
                .af6()
                .afrh11()
                .af9()
                .afrh12()
                .af9()
                .afrh15()
                .af6()
        });
        gpiob.afrl.modify(|_, w| w.afrl5().af6());
        gpioc.afrh.modify(|_, w| w.afrh10().af6().afrh11().af6());

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
                .ot8()
                .push_pull()
                .ot9()
                .push_pull()
                .ot10()
                .push_pull()
                .ot11()
                .push_pull()
                .ot12()
                .push_pull()
                .ot15()
                .push_pull()
        });
        gpiob
            .otyper
            .modify(|_, w| w.ot5().push_pull().ot9().push_pull());
        gpioc
            .otyper
            .modify(|_, w| w.ot6().push_pull().ot10().push_pull().ot11().push_pull());

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
                .ospeedr8()
                .very_high_speed()
                .ospeedr9()
                .very_high_speed()
                .ospeedr10()
                .very_high_speed()
                .ospeedr11()
                .very_high_speed()
                .ospeedr12()
                .very_high_speed()
                .ospeedr15()
                .very_high_speed()
        });
        gpiob
            .ospeedr
            .modify(|_, w| w.ospeedr5().very_high_speed().ospeedr9().very_high_speed());
        gpioc.ospeedr.modify(|_, w| {
            w.ospeedr6()
                .very_high_speed()
                .ospeedr10()
                .very_high_speed()
                .ospeedr11()
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
                .pupdr8()
                .floating()
                .pupdr9()
                .floating()
                .pupdr10()
                .floating()
                .pupdr11()
                .floating()
                .pupdr12()
                .floating()
                .pupdr15()
                .pull_up()
        });
        gpiob
            .pupdr
            .modify(|_, w| w.pupdr5().floating().pupdr9().floating());
        gpioc.pupdr.modify(|_, w| {
            w.pupdr6()
                .floating()
                .pupdr10()
                .floating()
                .pupdr11()
                .floating()
        });
    }

    fn configure_timers(&self) {
        // Configure TIM3 for 1kHz polling of SPI1
        let tim3 = &self.mode_state.tim3;
        // Stop the timer if it's running for some reason.
        tim3.cr1.modify(|_, w| w.cen().clear_bit());
        block_until!(tim3.cr1.read().cen().bit_is_clear());
        // Edge aligned mode, and up counting.
        tim3.cr1.modify(|_, w| w.dir().up().cms().edge_aligned());
        // Fire off a DMA on update (i.e. counter overflow)
        tim3.dier.modify(|_, w| w.ude().set_bit());
        // Assuming 170MHz core clock, set prescalar to 3 and ARR to 42500 for 170e6/42500/4=1kHz.
        // Why 3 and not 4? The timer clock is set to `core_clk / (PSC[PSC] + 1)`. If it were to use
        // the value directly it'd divide the clock by zero on reset, which would be A Bad Thing.
        // Safety: Upstream: This should have a proper range of 0-65535 in stm32-rs. 3 is well
        // within range.
        tim3.psc.write(|w| w.psc().bits(3));
        // Safety: Upstream: This should have a proper range of 0-65535 in stm32-rs. 42500 is within
        // range.
        tim3.arr.write(|w| unsafe { w.arr().bits(42500) });

        // Configure TIM3 for 40kHz control loop (80kHz frequency, since up + down = 1 full cycle).
        let tim1 = &self.mode_state.tim1;
        // Stop the timer if it's running for some reason.
        tim1.cr1.modify(|_, w| w.cen().clear_bit());
        block_until!(tim1.cr1.read().cen().bit_is_clear());
        // Center-aligned mode 2: Up/Down and interrupts on up only.
        tim1.cr1
            .modify(|_, w| w.dir().up().cms().center_aligned2().ckd().div1());
        // Enable output state low on idle.
        tim1.cr2.modify(|_, w| {
            w.ccpc()
                .clear_bit()
                .ois1()
                .clear_bit()
                .ois2()
                .clear_bit()
                .ois3()
                .clear_bit()
        });
        // TODO(blakely): enable interrupts
        // Configure output channels to PWM mode 1. Note: OCxM registers are split between the first
        // three bits and the fourth bit. For PWM mode 1 the fourth bit should be zero which is the
        // reset value, but it's good practice to manually set it anyway.
        tim1.ccmr1_output().modify(|_, w| {
            w.cc1s()
                .output()
                .oc1m()
                .pwm_mode1()
                .oc1m_3()
                .clear_bit()
                .cc2s()
                .output()
                .oc2m()
                .pwm_mode1()
                .oc2m_3()
                .clear_bit()
        });
        // Note: We don't actually want OC4 to be used, since it's only purpose is to trigger the
        // interrupt.
        tim1.ccmr2_output()
            .modify(|_, w| w.cc3s().output().oc3m().pwm_mode1().oc3m_3().clear_bit());
        // Enable channels 1-5. 1-3 are the output pins, 4 is the interrupt, and 5 is used as the
        // forced deadtime insertion. Set the output polarity to HIGH (rising edge).
        tim1.ccer.modify(|_, w| {
            w.cc1e()
                .set_bit()
                .cc1p()
                .clear_bit()
                .cc2e()
                .set_bit()
                .cc2p()
                .clear_bit()
                .cc3e()
                .set_bit()
                .cc3p()
                .clear_bit()
                .cc4e()
                .set_bit()
                .cc4p()
                .clear_bit()
                .cc5e()
                .set_bit()
                .cc5p()
                .clear_bit()
        });
        // 80kHz = Prescalar to 0, ARR to 2125
        tim1.psc.write(|w| w.psc().bits(0));
        // Safety: Upstream: needs range to be explicitly set for safety. 16-bit value.
        tim1.arr.write(|w| w.arr().bits(2125));
        // Set repetition counter to 1, since we only want update events on only after the full
        // up/down count cycle.
        // Safety: Upstream: needs range to be explicitly set for safety. 16-bit value.
        tim1.rcr.write(|w| unsafe { w.rep().bits(1) });
        // TODO(blakely): Actually use these in the control loop, but for now just set them to some
        // reasonable values.
        tim1.ccr1.write(|w| w.ccr1().bits(2125));
        tim1.ccr2.write(|w| w.ccr2().bits(1000));
        tim1.ccr3.write(|w| w.ccr3().bits(2083));
        // Safety: Upstream: needs range to be explicitly set for safety. 16-bit value.
        // Enable master output bit.
        tim1.bdtr.modify(|_, w| w.moe().set_bit());
        // Set ch5 to PWM mode and enable it.
        // Safety: Upstream: needs enum values. PWM mode 1 is 0110.
        tim1.ccmr3_output
            .modify(|_, w| unsafe { w.oc5m().bits(110).oc5m_bit3().bits(0) });
        // Configure channels 1-3 to be logical AND'd with channel 5, and set its capture compare
        // value.
        // Safety: Upstream: needs range to be explicitly set for safety.
        // TODO(blakely): Set this CCR to a logical safe PWM duty (min deadtime 400ns = 98.4% duty
        // cycle at 40kHz)
        tim1.ccr5.modify(|_, w| unsafe {
            w.gc5c1()
                .set_bit()
                .gc5c2()
                .set_bit()
                .gc5c3()
                .set_bit()
                .ccr5()
                .bits(2083)
        });
        // Enable interrupt for update
        tim1.dier.modify(|_, w| w.uie().set_bit());
    }

    fn configure_adcs(&self) {
        let adc4 = &self.mode_state.adc4;

        // Wake from deep power down, enable ADC voltage regulator, and set single-ended input mode.
        adc4.cr.modify(|_, w| {
            w.deeppwd()
                .disabled()
                .advregen()
                .enabled()
                .adcaldif()
                .single_ended()
        });

        // Begin calibration

        // TODO(blakely): calibration (MUST BE DONE AFTER DEEPPWD)
        // TODO(blakely): Single-ended (maybe before calib?)
        // Check ADRDY in ADC_ISR
    }

    pub fn configure_peripherals<'a>(self) -> Controller<Ready<impl DrvState>> {
        self.configure_gpio();
        self.configure_timers();
        self.configure_adcs();

        let ma702 = ma702::new(self.mode_state.spi1)
            .configure_spi()
            .begin_stream(&self.mode_state.dma1, &self.mode_state.dmamux);

        let gpioc = &self.mode_state.gpioc;
        let drv =
            drv8323rs::new(self.mode_state.spi3).enable(|| gpioc.bsrr.write(|w| w.bs6().set_bit()));

        // Configure DRV8323RS.
        use drv8323rs::registers::*;
        drv.control_register().update(|_, w| {
            w.pwm_mode()
                .variant(PwmMode::Pwm3x)
                .clear_latched_faults()
                .set_bit()
        });
        drv.current_sense().update(|_, w| {
            w.vref_divisor()
                .variant(CsaDivisor::Two)
                .current_sense_gain()
                .variant(CsaGain::V40)
                .sense_level()
                .variant(SenseOcp::V1)
        });

        // HACK HACK HACK
        // Disable drv for now while we work on TIM1. Don't want to accidentally short anything.
        let drv = drv.disable(|| gpioc.bsrr.write(|w| w.br6().set_bit()));

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
        // TIM1 Update
        enable_irq(device::Interrupt::TIM1_UP_TIM16);

        // Kick off tim1.
        self.mode_state.tim1.cr1.modify(|_, w| w.cen().set_bit());
        // Kick off tim3.
        self.mode_state.tim3.cr1.modify(|_, w| w.cen().set_bit());

        let new_self = Controller {
            mode_state: Ready {
                ma702,
                drv,
                gpioa: self.mode_state.gpioa,
                syst: self.mode_state.syst,
            },
        };
        new_self
    }
}

pub struct FdcanShared {
    pub sram: Sram,
    pub fdcan: device::FDCAN1,
}
