use crate::block_until;
use crate::block_while;
use crate::util::bitfield;
use core::marker::PhantomData;
use stm32g4::stm32g474 as device;

pub mod registers;
use registers::Addr;
use registers::*;

pub struct Drv8323rs<S> {
    spi: device::SPI3,

    #[allow(dead_code)]
    mode_state: S,
}

pub struct Init {}
pub struct Idle<EnSpi, DisSpi>
where
    EnSpi: Fn(),
    DisSpi: Fn(),
{
    enablEnSpi: EnSpi,
    disablEnSpi: DisSpi,
}

pub fn new<'a>(spi: device::SPI3) -> Drv8323rs<Init> {
    Drv8323rs {
        spi,
        mode_state: Init {},
    }
}

pub struct DrvRegister<'a, T: Addr, EnSpi: Fn(), DisSpi: Fn()> {
    spi: &'a device::SPI3,
    cs_low: &'a EnSpi,
    cs_high: &'a DisSpi,
    _marker: PhantomData<T>,
}

impl<'a, T: 'a + Addr, EnSpi: Fn(), DisSpi: Fn()> DrvRegister<'a, T, EnSpi, DisSpi> {
    pub fn read(&self) -> bitfield::ReadProxy<u16, T> {
        let spi = self.spi;
        let addr = T::addr();
        // (self.cs_low)();
        // Enable SPI
        spi.cr1.modify(|_, w| w.spe().set_bit());
        block_until! { spi.cr1.read().spe().bit_is_set() }
        spi.dr
            .write(|w| w.dr().bits((1u16 << 15) | (addr as u16) << 11));
        block_while! { spi.sr.read().bsy().bit_is_set() }
        // Disable SPI
        spi.cr1.modify(|_, w| w.spe().clear_bit());
        block_until! { spi.cr1.read().spe().bit_is_clear() }
        // (self.cs_high)();
        let bits = spi.dr.read().bits() as u16;
        let bits2 = spi.dr.read().bits() as u16;
        let bits3 = spi.dr.read().bits() as u16;

        bitfield::ReadProxy::<u16, T>::new(bits)
    }

    pub fn update<F>(&self, f: F)
    where
        F: FnOnce(
            &bitfield::ReadProxy<u16, T>,
            &mut bitfield::WriteProxy<u16, T>,
        ) -> &'a mut bitfield::WriteProxy<u16, T>,
    {
        let bits = self.read().bits;
        let value = f(
            &bitfield::ReadProxy::new(bits),
            &mut bitfield::WriteProxy::new(bits),
        )
        .bits;
        let spi = self.spi;
        let addr = T::addr();
        (self.cs_low)();
        spi.dr.write(|w| w.dr().bits(((addr as u16) << 11) | value));
        block_while! { spi.sr.read().bsy().bit_is_set() }
        // Each transaction requires a read/write to the SPI, so clear out the value from the `DR`
        // register.
        (self.cs_high)();
        let _ = spi.dr.read().bits();
    }
}

impl Drv8323rs<Init> {
    pub fn configure_spi<EnSpi: Fn(), DisSpi: Fn()>(
        self,
        enablEnSpi: EnSpi,
        disablEnSpi: DisSpi,
    ) -> Drv8323rs<Idle<EnSpi, DisSpi>> {
        // SPI config
        let spi = self.spi;

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
            mode_state: Idle {
                enablEnSpi,
                disablEnSpi,
            },
        }
    }
}

impl<'a, EnSpi: Fn(), DisSpi: Fn()> Drv8323rs<Idle<EnSpi, DisSpi>> {
    fn drv_register<RegType: Addr>(&'a self) -> DrvRegister<'a, RegType, EnSpi, DisSpi> {
        DrvRegister {
            spi: &self.spi,
            cs_low: &self.mode_state.enablEnSpi,
            cs_high: &self.mode_state.disablEnSpi,
            _marker: PhantomData,
        }
    }

    pub fn fault_status_1(
        &'a self,
    ) -> DrvRegister<'a, registers::fs1::FaultStatus1, EnSpi, DisSpi> {
        self.drv_register()
    }

    pub fn gate_drive_hs(
        &'a self,
    ) -> DrvRegister<'a, registers::gdhs::GateDriveHighSide, EnSpi, DisSpi> {
        DrvRegister {
            spi: &self.spi,
            cs_low: &self.mode_state.enablEnSpi,
            cs_high: &self.mode_state.disablEnSpi,
            _marker: PhantomData,
        }
    }
}
