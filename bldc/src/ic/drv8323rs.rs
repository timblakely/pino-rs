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
pub struct Idle {}

pub fn new(spi: device::SPI3) -> Drv8323rs<Init> {
    Drv8323rs {
        spi,
        mode_state: Init {},
    }
}

pub struct DrvRegister<'a, T: Addr> {
    spi: &'a device::SPI3,
    _marker: PhantomData<T>,
}

impl<'a, T: 'a + Addr> DrvRegister<'a, T> {
    pub fn read(&self) -> bitfield::ReadProxy<u16, T> {
        let spi = self.spi;
        let addr = T::addr();
        spi.dr
            .write(|w| w.dr().bits((1 << 15) & (addr as u16) << 11));
        block_until! { spi.sr.read().bsy().bit_is_set() }
        block_while! { spi.sr.read().bsy().bit_is_set() }
        let bits = spi.dr.read().bits() as u16;

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
        spi.dr.write(|w| w.dr().bits(((addr as u16) << 11) | value));
        block_until! { spi.sr.read().bsy().bit_is_set() }
        block_while! { spi.sr.read().bsy().bit_is_set() }
        // Each transaction requires a read/write to the SPI, so clear out the value from the `DR`
        // register.
        let _ = spi.dr.read().bits();
    }
}

impl Drv8323rs<Init> {
    pub fn configure_spi(self) -> Drv8323rs<Idle> {
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

        // Enable SPI
        spi.cr1.modify(|_, w| w.spe().set_bit());
        block_until! { spi.cr1.read().spe().bit_is_set() }

        Drv8323rs {
            spi,
            mode_state: Idle {},
        }
    }
}

impl Drv8323rs<Idle> {
    pub fn fault_status_1<'a>(&'a self) -> DrvRegister<'a, registers::fs1::FaultStatus1> {
        DrvRegister {
            spi: &self.spi,
            _marker: PhantomData,
        }
    }

    pub fn gate_drive_hs<'a>(&'a self) -> DrvRegister<'a, registers::gdhs::GateDriveHighSide> {
        DrvRegister {
            spi: &self.spi,
            _marker: PhantomData,
        }
    }
}
