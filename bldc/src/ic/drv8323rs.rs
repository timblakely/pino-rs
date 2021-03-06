use crate::block_until;
use crate::block_while;
use crate::util::bitfield;
use crate::util::stm32::blocking_sleep_us;
use core::marker::PhantomData;
use stm32g4::stm32g474 as device;

pub mod registers;
use registers::{
    Addr, CsaDivisor, CsaGain, DriveCurrent, OffsetCalibration, PwmMode, SenseOcp, SenseOvercurrent,
};

pub struct DrvRegister<'a, T: Addr> {
    spi: &'a device::SPI3,
    _marker: PhantomData<T>,
}

impl<'a, T: 'a + Addr> DrvRegister<'a, T> {
    pub fn read(&self) -> bitfield::ReadProxy<u16, T> {
        let spi = self.spi;
        let addr = T::addr();
        // Minimum of 400ns between frames
        blocking_sleep_us(1);

        // Enable SPI
        spi.cr1.modify(|_, w| w.spe().set_bit());
        block_until! { spi.cr1.read().spe().bit_is_set() }

        spi.dr
            .write(|w| w.dr().bits((1u16 << 15) | (addr as u16) << 11));
        block_while! { spi.sr.read().bsy().bit_is_set() }

        // Disable SPI
        spi.cr1.modify(|_, w| w.spe().clear_bit());
        block_until! { spi.cr1.read().spe().bit_is_clear() }

        let bits = spi.dr.read().bits() as u16;

        bitfield::ReadProxy::<u16, T>::new(bits)
    }

    pub fn update<F>(&'a self, f: F)
    where
        for<'w> F: FnOnce(
            &bitfield::ReadProxy<u16, T>,
            &'w mut bitfield::WriteProxy<u16, T>,
        ) -> &'w mut bitfield::WriteProxy<u16, T>,
    {
        let bits = self.read().bits;
        let value = f(
            &bitfield::ReadProxy::new(bits),
            &mut bitfield::WriteProxy::new(bits),
        )
        .bits;
        // TODO(blakely): Refactor this into a `write` function
        let spi = self.spi;
        let addr = T::addr();

        // Minimum of 400ns between frames
        blocking_sleep_us(1);
        // Enable SPI
        spi.cr1.modify(|_, w| w.spe().set_bit());
        block_until! { spi.cr1.read().spe().bit_is_set() }

        spi.dr.write(|w| w.dr().bits(((addr as u16) << 11) | value));
        block_while! { spi.sr.read().bsy().bit_is_set() }

        // Disable SPI
        spi.cr1.modify(|_, w| w.spe().clear_bit());
        block_until! { spi.cr1.read().spe().bit_is_clear() }

        // Each transaction requires a read/write to the SPI, so clear out the value from the `DR`
        // register.
        let _ = spi.dr.read().bits();
    }
}

pub struct Drv8323rs<S: DrvState> {
    spi: device::SPI3,
    #[allow(dead_code)]
    mode_state: S,
}

impl<'a, S: DrvState> Drv8323rs<S> {
    fn drv_register<RegType: Addr>(&'a self) -> DrvRegister<'a, RegType> {
        DrvRegister {
            spi: &self.spi,
            _marker: PhantomData,
        }
    }

    pub fn fault_status_1(&'a self) -> DrvRegister<'a, registers::fs1::FaultStatus1> {
        self.drv_register()
    }

    pub fn fault_status_2(&'a self) -> DrvRegister<'a, registers::fs2::FaultStatus2> {
        self.drv_register()
    }

    pub fn control_register(&'a self) -> DrvRegister<'a, registers::cr::ControlRegister> {
        self.drv_register()
    }

    pub fn gate_drive_hs(&'a self) -> DrvRegister<'a, registers::gdhs::GateDriveHighSide> {
        self.drv_register()
    }

    pub fn gate_drive_ls(&'a self) -> DrvRegister<'a, registers::gdls::GateDriveLowSide> {
        self.drv_register()
    }

    pub fn over_current_protection(
        &'a self,
    ) -> DrvRegister<'a, registers::ocp::OverCurrentProtection> {
        self.drv_register()
    }

    pub fn current_sense(&'a self) -> DrvRegister<'a, registers::csa::CurrentSenseAmplifier> {
        self.drv_register()
    }
}

pub trait DrvState {}

pub struct Sleep {}
impl DrvState for Sleep {}
pub struct Enabled {}
impl DrvState for Enabled {}

pub struct Ready {}
impl DrvState for Ready {}

pub fn new<'a>(spi: device::SPI3) -> Drv8323rs<Sleep> {
    // Disable SPI, if enabled.
    spi.cr1.modify(|_, w| w.spe().clear_bit());
    block_until! { spi.cr1.read().spe().bit_is_clear() }

    // Idle clock low, data capture on falling edge, transmission on rising edge
    // TODO(blakely): This assumes that the processor is running full bore at 170MHz
    spi.cr1.modify(|_, w| {
        w.cpha()
            .set_bit()
            .cpol()
            .clear_bit()
            .mstr()
            .set_bit()
            .br()
            .div128()
            .crcen()
            .clear_bit()
    });

    // 16 bit transfers
    spi.cr2.modify(|_, w| {
        w.ssoe()
            .enabled()
            .frf()
            .clear_bit()
            .ds()
            .sixteen_bit()
            .nssp()
            .set_bit()
    });

    Drv8323rs {
        spi,
        mode_state: Sleep {},
    }
}

impl Drv8323rs<Sleep> {
    pub fn enable<T: FnOnce()>(self, enable: T) -> Drv8323rs<Enabled> {
        (enable)();
        let new_drv = Drv8323rs {
            spi: self.spi,
            mode_state: Enabled {},
        };

        // Sleepy DRV8323's SPI port takes ~1ms to wake up.
        blocking_sleep_us(1000);
        // Make sure we can read the default bits after enabling.
        block_until! { new_drv.gate_drive_hs().read().bits == 1023 }
        new_drv
    }
}

impl<'a> Drv8323rs<Enabled> {
    pub fn calibrate(self) -> Drv8323rs<Ready> {
        // TODO(blakely): Move configuration up to higher level.
        self.control_register().update(|_, w| {
            w.disable_gate_drive_fault()
                .clear_bit()
                .disable_uvlo()
                .clear_bit()
                .pwm_mode()
                .variant(PwmMode::Pwm3x)
                .clear_latched_faults()
                .set_bit()
        });
        self.current_sense().update(|_, w| {
            w.vref_divisor()
                .variant(CsaDivisor::Two)
                .current_sense_gain()
                .variant(CsaGain::V40)
                .sense_level()
                .variant(SenseOcp::V1)
                .overcurrent_fault()
                .variant(SenseOvercurrent::Enabled)
        });
        // Begin ADC calibration. Requires >=100us
        self.current_sense().update(|_, w| {
            w.offset_calibration_a()
                .variant(OffsetCalibration::Calibration)
                .offset_calibration_b()
                .variant(OffsetCalibration::Calibration)
                .offset_calibration_c()
                .variant(OffsetCalibration::Calibration)
        });
        blocking_sleep_us(200);
        // Leave calibration mode
        self.current_sense().update(|_, w| {
            w.offset_calibration_a()
                .variant(OffsetCalibration::Normal)
                .offset_calibration_b()
                .variant(OffsetCalibration::Normal)
                .offset_calibration_c()
                .variant(OffsetCalibration::Normal)
        });

        // Use 1A drive current for FETs
        self.gate_drive_hs().update(|_, w| {
            w.idrive_p_high_side()
                .variant(DriveCurrent::Milli1000)
                .idrive_n_high_side()
                .variant(DriveCurrent::Milli1000)
        });
        self.gate_drive_ls().update(|_, w| {
            w.idrive_p_low_side()
                .variant(DriveCurrent::Milli1000)
                .idrive_n_low_side()
                .variant(DriveCurrent::Milli1000)
        });

        // Reset config after calibration
        self.current_sense().update(|_, w| {
            w.vref_divisor()
                .variant(CsaDivisor::Two)
                .current_sense_gain()
                .variant(CsaGain::V40)
                .sense_level()
                .variant(SenseOcp::V1)
                .overcurrent_fault()
                .variant(SenseOvercurrent::Enabled)
        });

        Drv8323rs {
            spi: self.spi,
            mode_state: Ready {},
        }
    }
}

impl<'a> Drv8323rs<Ready> {
    pub fn disable<T: FnOnce()>(self, disable: T) -> Drv8323rs<Sleep> {
        (disable)();
        Drv8323rs {
            spi: self.spi,
            mode_state: Sleep {},
        }
    }
}
