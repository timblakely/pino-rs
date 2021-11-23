use core::marker::PhantomData;

use crate::comms::fdcan::{self, FdcanMessage};
use crate::comms::fdcan::{Fdcan, Running};
use crate::commutation::calibrate_adc::CalibrateADC;
use crate::commutation::{Commutator, ControlHardware};
use crate::cordic::Cordic;
use crate::encoder::Encoder;
use crate::pwm::PwmOutput;
use crate::timer::TimerConfig;
use crate::util::stm32::{
    clock_setup, clocks::G4_CLOCK_SETUP, disable_dead_battery_pd, donate_systick,
};
use crate::{current_sensing, timer};
use crate::{ic::drv8323rs, ic::ma702};
use cortex_m::peripheral as cm;
use drv8323rs::Drv8323rs;
use enum_dispatch::enum_dispatch;
use stm32g4::stm32g474 as device;
use third_party::m4vga_rs::util::armv7m::{disable_irq, enable_irq};

const V_BUS_GAIN: f32 = 16.0; // 24v with a 150k/10k voltage divider.

pub struct Driver<S> {
    pub mode_state: S,
}

pub struct DriverHardware {
    pub drv: Drv8323rs<drv8323rs::Ready>,
    pub gpioa: device::GPIOA,
    pub fdcan: Fdcan<Running>,
}

pub struct Init {
    pub fdcan: device::FDCAN1,
    pub gpioa: device::GPIOA,
    pub gpiob: device::GPIOB,
    pub gpioc: device::GPIOC,
    pub spi1: device::SPI1,
    pub spi3: device::SPI3,
    pub tim1: device::TIM1,
    pub tim2: device::TIM2,
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

pub struct Calibrating {
    pub hardware: DriverHardware,
}

pub struct Ready {
    pub hardware: DriverHardware,
}

pub fn take_hardware() -> Driver<Init> {
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
        p.TIM2,
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
    tim2: device::TIM2,
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
) -> Driver<Init> {
    disable_dead_battery_pd(&pwr);

    // Make sure we don't receive any interrupts before we're ready.
    disable_irq(device::Interrupt::ADC1_2);
    disable_irq(device::Interrupt::DMA1_CH2);
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
        .modify(|_, w| w.fdcanen().enabled().tim2en().enabled().tim3en().enabled());
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
        // Next is the DMA request reading the MA702 (if used).
        nvic.set_priority(device::Interrupt::DMA1_CH2, 0x10);
        // The periodic callback on TIM2 is low priority.
        nvic.set_priority(device::Interrupt::TIM2, 0xFF);
        // Finally the FDCAN.
        nvic.set_priority(device::Interrupt::FDCAN1_INTR0_IT, 0xFF);
        nvic.set_priority(device::Interrupt::FDCAN1_INTR1_IT, 0xFF);
    }

    Driver {
        mode_state: Init {
            fdcan,
            gpioa,
            gpiob,
            gpioc,
            spi1,
            spi3,
            tim1,
            tim2,
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

impl Driver<Init> {
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
        // PB6 - LED 2
        // PB7 - LED 3
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
                .moder6()
                .output()
                .moder7()
                .output()
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
        gpiob.otyper.modify(|_, w| {
            w.ot5()
                .push_pull()
                .ot6()
                .push_pull()
                .ot7()
                .push_pull()
                .ot9()
                .push_pull()
        });
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
        gpiob.ospeedr.modify(|_, w| {
            w.ospeedr5()
                .very_high_speed()
                .ospeedr6()
                .very_high_speed()
                .ospeedr7()
                .very_high_speed()
                .ospeedr9()
                .very_high_speed()
        });
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
        gpiob.pupdr.modify(|_, w| {
            w.pupdr5()
                .floating()
                .pupdr6()
                .floating()
                .pupdr7()
                .floating()
                .pupdr9()
                .floating()
        });
        gpioc.pupdr.modify(|_, w| {
            w.pupdr6()
                .floating()
                .pupdr10()
                .floating()
                .pupdr11()
                .floating()
        });
    }

    pub fn configure_peripherals<'a>(self) -> Driver<Calibrating> {
        self.configure_gpio();
        let pwm = PwmOutput::new(self.mode_state.tim1, true).configure(TimerConfig {
            prescalar: 1,
            arr: 2125,
        });

        let ma702 = ma702::new(self.mode_state.spi1, self.mode_state.tim3)
            .configure_spi()
            .begin_stream_polling(self.mode_state.dma1, &self.mode_state.dmamux);

        let encoder = Encoder::new(ma702, 21, 200.);

        let gpioc = &self.mode_state.gpioc;
        let drv = drv8323rs::new(self.mode_state.spi3)
            .enable(|| gpioc.bsrr.write(|w| w.bs6().set_bit()))
            .calibrate();

        timer::donate_hardware_for_scheduler(self.mode_state.tim2);

        timer::periodic_callback(1000., 0.001, || {
            let _asdf = 1;
        });

        // Set up current sensing.
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
        let current_sensor = current_sensing::new(
            self.mode_state.adc1,
            self.mode_state.adc2,
            self.mode_state.adc3,
            self.mode_state.adc4,
            self.mode_state.adc5,
        )
        .configure_phase_sensing()
        .configure_v_refint()
        .configure_v_bus(V_BUS_GAIN)
        .ready();

        // Configure FDCAN
        let fdcan = fdcan::take(self.mode_state.fdcan)
            // TODO(blakely): clean up this API.
            .set_extended_filter(
                0,
                fdcan::extended_filter::ExtendedFilterMode::StoreRxFIFO0,
                fdcan::extended_filter::ExtendedFilterType::Classic,
                0x1,
                0xFFF_FFFF,
            )
            .configure_interrupts()
            .configure_protocol()
            .configure_timing()
            .fifo_mode()
            .start();

        // Tx IRQ
        enable_irq(device::Interrupt::FDCAN1_INTR0_IT);
        // Rx IRQ
        enable_irq(device::Interrupt::FDCAN1_INTR1_IT);
        // ADC1 IRQ
        enable_irq(device::Interrupt::ADC1_2);

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

        Commutator::donate_hardware(ControlHardware {
            current_sensor: current_sensor,
            pwm,
            encoder,
            cordic: Cordic::new(cordic, 20),
        });

        Driver {
            mode_state: Calibrating {
                hardware: DriverHardware {
                    drv,
                    gpioa: self.mode_state.gpioa,
                    fdcan,
                },
            },
        }
    }
}

impl Driver<Calibrating> {
    pub fn calibrate(self) -> Driver<Ready> {
        Commutator::enable_loop();
        Commutator::set(CalibrateADC::new(2., move |_| {}).into());
        while Commutator::is_enabled() {}
        Commutator::disable_loop();

        Driver {
            mode_state: Ready {
                hardware: self.mode_state.hardware,
            },
        }
    }
}

impl Driver<Ready> {
    pub fn listen(mut self) -> ! {
        Commutator::enable_loop();

        loop {
            let fdcan = &mut self.mode_state.hardware.fdcan;
            fdcan.process_messages();
        }
    }

    pub fn on_message(&mut self, message_handler: fn(FdcanMessage)) {
        self.mode_state
            .hardware
            .fdcan
            .message_handler(message_handler);
    }
}
