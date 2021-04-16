use crate::block_until;
use crate::block_while;
use crate::util::bitfield;
use core::marker::PhantomData;
use stm32g4::stm32g474 as device;

pub mod registers;
use registers::Addr;

pub struct DrvRegister<'a, T: Addr> {
    spi: &'a device::SPI3,
    _marker: PhantomData<T>,
}

impl<'a, T: 'a + Addr> DrvRegister<'a, T> {
    pub fn read(&self) -> bitfield::ReadProxy<u16, T> {
        let spi = self.spi;
        let addr = T::addr();

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

pub struct Drv8323rs<S, E: Fn(), D: Fn()> {
    enable: E,
    disable: D,

    #[allow(dead_code)]
    mode_state: S,
}

pub struct Sleep {
    spi: device::SPI3,
}
pub struct Enabled {
    spi: device::SPI3,
}

pub fn new<'a, E: Fn(), D: Fn()>(
    spi: device::SPI3,
    enable: E,
    disable: D,
) -> Drv8323rs<Sleep, E, D> {
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
        enable,
        disable,
        mode_state: Sleep { spi },
    }
}

impl<E: Fn(), D: Fn()> Drv8323rs<Sleep, E, D> {
    pub fn enable(self) -> Drv8323rs<Enabled, E, D> {
        (self.enable)();
        let new_drv = Drv8323rs {
            enable: self.enable,
            disable: self.disable,
            mode_state: Enabled {
                spi: self.mode_state.spi,
            },
        };
        // SPI port takes ~1ms to wake up. Poll until we get a reset value we expect.
        // TODO(blakely): I don't like blocking here, but don't want to sacrifice a timer or enable
        // SysTick. Figure out something better.
        block_until! { new_drv.gate_drive_hs().read().bits == 1023 }
        new_drv
    }
}

impl<'a, E: Fn(), D: Fn()> Drv8323rs<Enabled, E, D> {
    fn drv_register<RegType: Addr>(&'a self) -> DrvRegister<'a, RegType> {
        DrvRegister {
            spi: &self.mode_state.spi,
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

    pub fn disable(self) -> Drv8323rs<Sleep, E, D> {
        (self.disable)();
        Drv8323rs {
            enable: self.enable,
            disable: self.disable,
            mode_state: Sleep {
                spi: self.mode_state.spi,
            },
        }
    }
}
