use crate::comms::fdcan::FdcanMessage;
use crate::commutation::{ControlParameters, Hardware};
use crate::util::buffered_state::{BufferedState, StateReader};
use crate::util::stm32::{
    blocking_sleep_us, clock_setup, clocks::G4_CLOCK_SETUP, disable_dead_battery_pd, donate_systick,
};
use crate::{
    block_until,
    comms::fdcan::{self, Sram, FDCANSHARE, FDCAN_RECEIVE_BUF},
};
use crate::{
    block_while,
    ic::drv8323rs,
    ic::ma702::{self, Ma702, Streaming},
};
use cortex_m::peripheral as cm;
use drv8323rs::Drv8323rs;
use fixed::types::I1F31;
use ringbuffer::RingBufferRead;
use stm32g4::stm32g474::{self as device, interrupt};
use third_party::m4vga_rs::util::armv7m::{clear_pending_irq, disable_irq, enable_irq};
use third_party::m4vga_rs::util::spin_lock::{SpinLock, SpinLockGuard};

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
    pub adc12: device::ADC12_COMMON,
    pub adc1: device::ADC1,
    pub adc2: device::ADC2,
    pub adc345: device::ADC345_COMMON,
    pub adc3: device::ADC3,
    pub adc4: device::ADC4,
    pub adc5: device::ADC5,
    pub cordic: device::CORDIC,
}

pub struct Ready {
    pub ma702: Ma702<Streaming>,
    pub drv: Drv8323rs<drv8323rs::Ready>,
    pub gpioa: device::GPIOA,
    pub tim1: device::TIM1,
    pub adcs: (
        device::ADC1,
        device::ADC2,
        device::ADC3,
        device::ADC4,
        device::ADC5,
    ),
}

pub fn take_hardware() -> Controller<Init> {
    let cp = cm::Peripherals::take().unwrap();
    let p = device::Peripherals::take().unwrap();

    // Donate the SYST peripheral to the blocking sleep handler so it's available anywhere.
    donate_systick(cp.SYST);

    init(
        cp.NVIC,
        p.RCC,
        p.FLASH,
        p.PWR,
        p.GPIOA,
        p.GPIOB,
        p.GPIOC,
        p.FDCAN1,
        p.SPI1,
        p.SPI3,
        p.TIM1,
        p.TIM3,
        p.DMA1,
        p.DMAMUX,
        p.ADC12_COMMON,
        p.ADC1,
        p.ADC2,
        p.ADC345_COMMON,
        p.ADC3,
        p.ADC4,
        p.ADC5,
        p.CORDIC,
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
    adc12: device::ADC12_COMMON,
    adc1: device::ADC1,
    adc2: device::ADC2,
    adc345: device::ADC345_COMMON,
    adc3: device::ADC3,
    adc4: device::ADC4,
    adc5: device::ADC5,
    cordic: device::CORDIC,
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
    rcc.ahb1enr.modify(|_, w| {
        w.dma1en()
            .enabled()
            .dmamuxen()
            .enabled()
            .cordicen()
            .set_bit()
    });
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
        // Ensure that the control loop is at the absolute highest priority.
        nvic.set_priority(device::Interrupt::ADC1_2, 0x0);
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
            adc12,
            adc1,
            adc2,
            adc345,
            adc3,
            adc4,
            adc5,
            cordic,
        },
    }
}

impl Controller<Init> {
    // TODO(blakely): Move into a device-specific, feature-guarded trait
    fn configure_gpio(&self) {
        // TODO(blakely): Implement split-borrowing to allow devices to take their own pins.
        // Configure GPIOA pins
        // PA0 - ADC2_IN1 - SENSE_B
        // PA1 - ADC1_IN2 - SENSE_A
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
        // PB1 - ADC3_IN1 - SENSE_C
        // PB5 - SPI3 - DRV_MOSI - AF6
        // PB9 - LED 1
        // PB12 - ADC4_IN3 - SENSE_BAT
        // PC6 - DRV_ENABLE
        // PC10 - SPI3 - DRV_SCK - AF6
        // PC11 - SPI3 - DRV_MISO - AF6
        let gpioa = &self.mode_state.gpioa;
        let gpiob = &self.mode_state.gpiob;
        let gpioc = &self.mode_state.gpioc;

        // Pin modes
        gpioa.moder.modify(|_, w| {
            w.moder0()
                .analog()
                .moder1()
                .analog()
                .moder4()
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
        gpiob.moder.modify(|_, w| {
            w.moder1()
                .analog()
                .moder5()
                .alternate()
                .moder9()
                .output()
                .moder12()
                .analog()
        });
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
        // Assuming 170MHz core clock, set prescalar to 4 and ARR to 42500 for 170e6/42500/4=1kHz.
        // Why is the value actually 3 and not 4? The timer clock is set to `core_clk / (PSC[PSC] +
        // 1)`. If it were to use the value directly it'd divide the clock by zero on reset, which
        // would be A Bad Thing.
        // Safety: Upstream: This should have a proper range of 0-65535 in stm32-rs. 3 is well
        // within range.
        tim3.psc.write(|w| w.psc().bits(3));
        // Safety: Upstream: This should have a proper range of 0-65535 in stm32-rs. 42500 is within
        // range.
        tim3.arr.write(|w| unsafe { w.arr().bits(42500) });

        // Configure TIM1 for 40kHz control loop (80kHz frequency, since up + down = 1 full cycle).
        let tim1 = &self.mode_state.tim1;
        // Stop the timer if it's running for some reason.
        tim1.cr1.modify(|_, w| w.cen().clear_bit());
        block_until!(tim1.cr1.read().cen().bit_is_clear());
        // Center-aligned mode 2: Up/Down and interrupts on up only.
        tim1.cr1
            .modify(|_, w| w.dir().up().cms().center_aligned2().ckd().div1());
        // Enable output state low on idle. Also set the master mode so that trgo2 is written based
        // on `tim_oc4refc`
        // Safety: mms2 doesn't have a valid range or enum set. Bits 0b0111 are tim_oc4refc.
        tim1.cr2.modify(|_, w| {
            unsafe {
                w.ccpc()
                    .clear_bit()
                    .ois1()
                    .clear_bit()
                    .ois2()
                    .clear_bit()
                    .ois3()
                    .clear_bit()
                    .ois4()
                    .clear_bit()
                    // Configure tim_oc4refc to be on ch4. Note that this must be on mms2 for trgo2!
                    .mms2()
                    .bits(0b0111)
            }
        });
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
        tim1.ccmr2_output().modify(|_, w| {
            w.cc3s()
                .output()
                .oc3m()
                .pwm_mode1()
                .oc3m_3()
                .clear_bit()
                .cc4s()
                .output()
                .oc4m()
                .pwm_mode1()
                .oc4m_3()
                .clear_bit()
        });
        // Enable channels 1-5. 1-3 are the output pins, channel 4 is used to trigger the current
        // sampling, and 5 is used as the forced deadtime insertion. Set the output polarity to HIGH
        // (rising edge).
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
        // 80kHz@170MHz = Prescalar to 0, ARR to 2125
        tim1.psc.write(|w| w.psc().bits(0));
        tim1.arr.write(|w| w.arr().bits(2125));
        // Set repetition counter to 1, since we only want update TIM1 events on only after the full
        // up/down count cycle.
        // Safety: Upstream: needs range to be explicitly set for safety. 16-bit value.
        tim1.rcr.write(|w| unsafe { w.rep().bits(1) });
        tim1.ccr1.write(|w| w.ccr1().bits(0));
        tim1.ccr2.write(|w| w.ccr2().bits(0));
        tim1.ccr3.write(|w| w.ccr3().bits(0));
        // Set channel 4 to trigger _just_ before the midway point.
        tim1.ccr4.write(|w| w.ccr4().bits(2124));
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
    }

    fn configure_adcs(&self) {
        let adc1 = &self.mode_state.adc1;
        let adc2 = &self.mode_state.adc2;
        let adc3 = &self.mode_state.adc3;
        let adc4 = &self.mode_state.adc4;
        let adc5 = &self.mode_state.adc5;
        // Begin in a sane state.
        adc1.cr.modify(|_, w| {
            w.adcal()
                .clear_bit()
                .aden()
                .clear_bit()
                .adstart()
                .clear_bit()
                .advregen()
                .clear_bit()
        });
        adc2.cr.modify(|_, w| {
            w.adcal()
                .clear_bit()
                .aden()
                .clear_bit()
                .adstart()
                .clear_bit()
                .advregen()
                .clear_bit()
        });
        adc3.cr.modify(|_, w| {
            w.adcal()
                .clear_bit()
                .aden()
                .clear_bit()
                .adstart()
                .clear_bit()
                .advregen()
                .clear_bit()
        });
        adc4.cr.modify(|_, w| {
            w.adcal()
                .clear_bit()
                .aden()
                .clear_bit()
                .adstart()
                .clear_bit()
                .advregen()
                .clear_bit()
        });
        adc5.cr.modify(|_, w| {
            w.adcal()
                .clear_bit()
                .aden()
                .clear_bit()
                .adstart()
                .clear_bit()
                .advregen()
                .clear_bit()
        });

        // Set up the ADC clocks. We're assuming we're running on a 170MHz AHB bus, so div=4 gives
        // us 42.5MHz (below max freq of 60MHz for single or 52MHz for multiple channels).
        self.mode_state
            .adc12
            .ccr
            .modify(|_, w| w.ckmode().sync_div4());
        self.mode_state.adc345.ccr.modify(|_, w| {
            w.ckmode()
                .sync_div4()
                // Bring up the Vref channel for ADC5
                .vrefen()
                .set_bit()
        });

        // Wake from deep power down, enable ADC voltage regulator, and set single-ended input mode.
        adc1.cr
            .modify(|_, w| w.deeppwd().disabled().advregen().enabled());
        adc2.cr
            .modify(|_, w| w.deeppwd().disabled().advregen().enabled());
        adc3.cr
            .modify(|_, w| w.deeppwd().disabled().advregen().enabled());
        adc4.cr
            .modify(|_, w| w.deeppwd().disabled().advregen().enabled());
        adc5.cr
            .modify(|_, w| w.deeppwd().disabled().advregen().enabled());

        // Allow voltage regulators to warm up. Datasheet says 20us max.
        blocking_sleep_us(20);

        // Begin calibration
        // Can probably combine these modifies, but kept separate in case the clear bit has to be
        // set first.
        adc1.cr.modify(|_, w| w.aden().clear_bit());
        adc1.cr.modify(|_, w| w.adcaldif().single_ended());
        adc1.cr.modify(|_, w| w.adcal().set_bit());
        adc2.cr.modify(|_, w| w.aden().clear_bit());
        adc2.cr.modify(|_, w| w.adcaldif().single_ended());
        adc2.cr.modify(|_, w| w.adcal().set_bit());
        adc3.cr.modify(|_, w| w.aden().clear_bit());
        adc3.cr.modify(|_, w| w.adcaldif().single_ended());
        adc3.cr.modify(|_, w| w.adcal().set_bit());
        adc4.cr.modify(|_, w| w.aden().clear_bit());
        adc4.cr.modify(|_, w| w.adcaldif().single_ended());
        adc4.cr.modify(|_, w| w.adcal().set_bit());
        adc5.cr.modify(|_, w| w.aden().clear_bit());
        adc5.cr.modify(|_, w| w.adcaldif().single_ended());
        adc5.cr.modify(|_, w| w.adcal().set_bit());
        // Wait for it to complete
        block_until!(adc1.cr.read().adcal().bit_is_clear());
        block_until!(adc2.cr.read().adcal().bit_is_clear());
        block_until!(adc3.cr.read().adcal().bit_is_clear());
        block_until!(adc4.cr.read().adcal().bit_is_clear());
        block_until!(adc5.cr.read().adcal().bit_is_clear());

        // Check that we're ready, enable, and wait for ready state. Initial adrdy.set_bit is to
        // ensure it's cleared.
        adc1.isr.modify(|_, w| w.adrdy().set_bit());
        adc1.cr.modify(|_, w| w.aden().set_bit());
        adc2.isr.modify(|_, w| w.adrdy().set_bit());
        adc2.cr.modify(|_, w| w.aden().set_bit());
        adc3.isr.modify(|_, w| w.adrdy().set_bit());
        adc3.cr.modify(|_, w| w.aden().set_bit());
        adc4.isr.modify(|_, w| w.adrdy().set_bit());
        adc4.cr.modify(|_, w| w.aden().set_bit());
        adc5.isr.modify(|_, w| w.adrdy().set_bit());
        adc5.cr.modify(|_, w| w.aden().set_bit());
        // Wait for ready
        block_until!(adc1.isr.read().adrdy().bit_is_set());
        block_until!(adc2.isr.read().adrdy().bit_is_set());
        block_until!(adc3.isr.read().adrdy().bit_is_set());
        block_until!(adc4.isr.read().adrdy().bit_is_set());
        block_until!(adc5.isr.read().adrdy().bit_is_set());
        // Clear ready, for good measure.
        adc1.isr.modify(|_, w| w.adrdy().set_bit());
        adc2.isr.modify(|_, w| w.adrdy().set_bit());
        adc3.isr.modify(|_, w| w.adrdy().set_bit());
        adc4.isr.modify(|_, w| w.adrdy().set_bit());
        adc5.isr.modify(|_, w| w.adrdy().set_bit());

        // Configure channels

        // ADC[123] - Current sense amplifiers. Single channel inputs, and triggered by `tim_trgo2`.
        adc1.cr.modify(|_, w| w.adstart().clear_bit());
        adc2.cr.modify(|_, w| w.adstart().clear_bit());
        adc3.cr.modify(|_, w| w.adstart().clear_bit());
        // Note that L=0 implies 1 conversion.
        // Safety: SVD doesn't have valid range for this, so we're "arbitrarily setting bits". As
        // long as it's 0-16 for L and 0-18 for SQx, we should be good.
        adc1.sqr1
            .modify(|_, w| unsafe { w.l().bits(0).sq1().bits(2) });
        adc2.sqr1
            .modify(|_, w| unsafe { w.l().bits(0).sq1().bits(1) });
        adc3.sqr1
            .modify(|_, w| unsafe { w.l().bits(0).sq1().bits(1) });
        // Fastest sample time we can, since there should be little-to-no resistance coming in from
        // the DRV current sense amplifier.
        adc1.smpr1.modify(|_, w| w.smp2().cycles2_5());
        adc2.smpr1.modify(|_, w| w.smp1().cycles2_5());
        adc3.smpr1.modify(|_, w| w.smp1().cycles2_5());
        // 12-bit non-continuous conversion (triggered).
        adc1.cfgr.modify(|_, w| {
            w.res()
                .bits12()
                .exten()
                .rising_edge()
                .extsel()
                .tim1_trgo2()
                .align()
                .right()
                .cont()
                .single()
                .discen()
                .disabled()
                .ovrmod()
                .overwrite()
        });
        adc2.cfgr.modify(|_, w| {
            w.res()
                .bits12()
                .exten()
                .rising_edge()
                .extsel()
                .tim1_trgo2()
                .align()
                .right()
                .cont()
                .single()
                .discen()
                .disabled()
                .ovrmod()
                .overwrite()
        });
        adc3.cfgr.modify(|_, w| {
            w.res()
                .bits12()
                .exten()
                .rising_edge()
                .extsel()
                .tim1_trgo2()
                .align()
                .right()
                .cont()
                .single()
                .discen()
                .disabled()
                .ovrmod()
                .overwrite()
        });
        // Enable interrupt on ADC1 EOS. Only needed for ADC1, since 2 and 3 are sync'd to the same
        // tim_trgo2.
        adc1.ier.modify(|_, w| w.eosie().enabled());
        // Start sampling.
        adc1.cr.modify(|_, w| w.adstart().set_bit());
        adc2.cr.modify(|_, w| w.adstart().set_bit());
        adc3.cr.modify(|_, w| w.adstart().set_bit());

        // ADC4
        // ADC4 only uses a single channel: IN3
        // Safety: SVD doesn't have valid range for this, so we're "arbitrarily setting bits". As
        // long as it's 0-16 for L and 0-18 for SQx, we should be good.
        adc4.cr.modify(|_, w| w.adstart().clear_bit());
        adc4.sqr1
            .modify(|_, w| unsafe { w.l().bits(0).sq1().bits(3) });
        // There's quite a bit of input resistance on the Vbus line. Datasheet suggests 39kOhm is
        // the upper limit for 60MHz sampling. We're using 42.5 and doing a single channel, so we
        // should be somewhat clear sampling for longer.
        adc4.smpr1.modify(|_, w| w.smp3().cycles640_5());
        // Set 12-bit continuous conversion mode with right-data-alignment, and ensure that no
        // hardware trigger is used. Also set overrun mode to allow overwrites of the data register,
        // otherwise it'll pause after one.
        adc4.cfgr.modify(|_, w| {
            w.res()
                .bits12()
                .cont()
                .continuous()
                .align()
                .right()
                .exten()
                .disabled()
                .ovrmod()
                .overwrite()
        });
        // Here goes nothin'... start it up.
        adc4.cr.modify(|_, w| w.adstart().set_bit());

        // ADC5 - Similar to ADC4 above, but using IN18
        adc5.cr.modify(|_, w| w.adstart().clear_bit());
        adc5.sqr1
            .modify(|_, w| unsafe { w.l().bits(0).sq1().bits(18) });
        adc5.smpr2.modify(|_, w| w.smp18().cycles640_5());
        adc5.cfgr.modify(|_, w| {
            w.res()
                .bits12()
                .cont()
                .set_bit()
                .align()
                .right()
                .exten()
                .disabled()
                .ovrmod()
                .overwrite()
        });
        adc5.cr.modify(|_, w| w.adstart().set_bit());
    }

    pub fn configure_peripherals<'a>(self) -> Controller<Ready> {
        self.configure_gpio();
        self.configure_timers();
        self.configure_adcs();

        let ma702 = ma702::new(self.mode_state.spi1)
            .configure_spi()
            .begin_stream(&self.mode_state.dma1, &self.mode_state.dmamux);

        let gpioc = &self.mode_state.gpioc;
        let drv = drv8323rs::new(self.mode_state.spi3)
            .enable(|| gpioc.bsrr.write(|w| w.bs6().set_bit()))
            .calibrate();

        // Configure FDCAN
        let fdcan = fdcan::take(self.mode_state.fdcan)
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
        *FDCANSHARE.try_lock().unwrap() = Some(fdcan.donate());

        fdcan::init_receive_buf();

        // Tx IRQ
        enable_irq(device::Interrupt::FDCAN1_INTR0_IT);
        // Rx IRQ
        enable_irq(device::Interrupt::FDCAN1_INTR1_IT);
        // ADC1 IRQ
        enable_irq(device::Interrupt::ADC1_2);

        // Kick off tim3.
        self.mode_state.tim3.cr1.modify(|_, w| w.cen().set_bit());

        let cordic = self.mode_state.cordic;
        // Safety: yet another SVD range missing. Valid ranges for precision is 1-15
        cordic.csr.modify(|_, w| unsafe {
            w.func()
                .cosine()
                .precision()
                // 20 iterations / 4 = 5 cycles
                .bits(5)
                .nres()
                .num2()
                .nargs()
                .num1()
                .ressize()
                .bits32()
                .argsize()
                .bits32()
        });

        // TODO(blakely): Move this into the commutation code.
        // Try it out
        // Note that the input to the CORDIC is theta/pi. Kinda nice in a way...
        let pi_over_3: I1F31 = I1F31::from_num(1f32 / 3f32);
        // Safety: Needs valid range in SVD. Supports full range of Q1.31 [-1,1-2^-31]
        cordic
            .wdata
            .write(|w| unsafe { w.bits(pi_over_3.to_bits() as u32) });
        block_until!(cordic.csr.read().rrdy().is_ready());
        let _cos: f32 = I1F31::from_bits(cordic.rdata.read().bits() as i32).to_num();
        let _sin: f32 = I1F31::from_bits(cordic.rdata.read().bits() as i32).to_num();

        let new_self = Controller {
            mode_state: Ready {
                ma702,
                drv,
                gpioa: self.mode_state.gpioa,
                tim1: self.mode_state.tim1,
                adcs: (
                    self.mode_state.adc1,
                    self.mode_state.adc2,
                    self.mode_state.adc3,
                    self.mode_state.adc4,
                    self.mode_state.adc5,
                ),
            },
        };
        new_self
    }
}

static CONTROL: third_party::m4vga_rs::util::iref::IRef<(&mut Hardware, &ControlParameters)> =
    third_party::m4vga_rs::util::iref::IRef::new();
static COMMUTATION_SHARED: SpinLock<Option<(Hardware, StateReader<ControlParameters>)>> =
    SpinLock::new(None);
static COMMS_SHARED: SpinLock<Option<BufferedState<ControlParameters>>> = SpinLock::new(None);

// TODO(blakely): implement Controller<Silent> for the state prior to comms setup.
impl Controller<Ready> {
    pub fn run(
        self,
        initial_state: ControlParameters,
        mut comms_handler: impl FnMut(&FdcanMessage, &mut ControlParameters),
        mut commutation_impl: impl FnMut(&mut Hardware, &ControlParameters) + Send,
    ) {
        // Since the interrupt handler can interrupt the main thread's modifications of any shared
        // state, we use a double-buffer and atomic swap to ensure a full update.
        *COMMS_SHARED.try_lock().unwrap() =
            Some(BufferedState::<ControlParameters>::new(initial_state));
        // Split the two states into a read-only control state that has read priority, and a mutable
        // state used for communication that writes the unused state and atomically swaps on
        // completion.
        let mut shared_state = SpinLockGuard::map(
            COMMS_SHARED.try_lock().expect("Shared state lock held"),
            |o| o.as_mut().expect("Shared state not initialized"),
        );
        let (control_params, mut comms_params) = shared_state.split();

        // Pass off both the control parameters and hardware to the commutation interrupt handler.
        {
            let mut shared = COMMUTATION_SHARED.lock();
            *shared = Some((
                Hardware {
                    tim1: self.mode_state.tim1,
                    ma702: self.mode_state.ma702,
                    adcs: self.mode_state.adcs,
                    sign: -1.,
                    square_wave_state: 0,
                },
                control_params,
            ));
        }
        // We don't want the timer to fire while we've got this lock, so disable the interrupt while
        // we're starting it up.
        crate::util::interrupts::free_from(
            device::interrupt::ADC1_2,
            &COMMUTATION_SHARED,
            |shared| {
                let (hardware, _) = &*shared;
                // Kick off tim1.
                hardware.tim1.cr1.modify(|_, w| w.cen().set_bit());
                // Now that the timer has started, enable the main output to allow current on the pins. If
                // we do this before we enable the time, we have the potential to get into a state where the
                // PWM pins are in an active state but the timer isn't running, potentially drawing tons of
                // current through the high phase to any low phases.
                hardware.tim1.bdtr.modify(|_, w| w.moe().set_bit());
            },
        );

        //

        // This is where the fun starts. We've got the commutation function here in the main loop,
        // but we've got to send it to the interrupt handler. Callbacks are fat pointers, and so far
        // you're only allowed to store trait objects (`dyn FnMut`) in `Box`es. We're getting around
        // that by donating it to an `IRef` thats' accessible from the ADC1_2 interrupt handler. The
        // way `IRef` works is that it donates the reference only as long as the `scope` parameter
        // is not finished. Once `scope` returns, it will wait for any observers to complete then to
        // the empty state.
        CONTROL.donate(&mut commutation_impl, || {
            loop {
                // Not only do we lock the receive buffer, but we prevent the FDCAN_INTR1 from
                // firing - the only other interrupt that shares this particular buffer - ensuring
                // we aren't preempted when reading from it. This is fine in general since the
                // peripheral itself has an internal buffer, and as long as we can clear the backlog
                // before the peripheral receives 4 requests we should be good. Alternatively, we
                // could just process a single message here to make sure that we only hold this lock
                // for the absolute minimum time, since there's an internal buffer in the FDCAN. Bad
                // form though...
                crate::util::interrupts::free_from(
                    device::interrupt::FDCAN1_INTR1_IT,
                    &FDCAN_RECEIVE_BUF,
                    |mut buf| {
                        while let Some(message) = buf.dequeue_ref() {
                            comms_handler(&message, &mut comms_params.update());
                        }
                    },
                );
            }
        });
    }
}

#[interrupt]
fn ADC1_2() {
    // Main control loop.
    unsafe {
        *(0x4800_0418 as *mut u32) = 1 << 9;
        cortex_m::asm::nop();
        cortex_m::asm::nop();
        cortex_m::asm::nop();
        cortex_m::asm::nop();
        cortex_m::asm::nop();
        cortex_m::asm::nop();
        cortex_m::asm::nop();
        cortex_m::asm::nop();
        cortex_m::asm::nop();
        cortex_m::asm::nop();
        *(0x4800_0418 as *mut u32) = 1 << (9 + 16);
    }
    // HACK HACK HACK: Clear EOS for ADC 1
    unsafe {
        *(0x5000_0000 as *mut u32) = 1 << 3;
    }
    let (hardware, control_parameters) = &mut *SpinLockGuard::map(
        COMMUTATION_SHARED.try_lock().expect("adc interrupt lock"),
        |o| o.as_mut().expect("Control params not set"),
    );
    CONTROL.observe(|r| r(hardware, control_parameters.read()));
    clear_pending_irq(device::Interrupt::ADC1_2);
}

pub struct FdcanShared {
    pub sram: Sram,
    pub fdcan: device::FDCAN1,
}
