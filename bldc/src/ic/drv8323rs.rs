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

impl<'a, T: Addr> DrvRegister<'a, T> {
    pub fn read(&self) -> bitfield::ReadProxy<u16, T> {
        let spi = self.spi;
        // let addr = Self::addr();
        let addr = T::addr();
        spi.dr.write(|w| w.dr().bits((addr as u16) << 15));
        block_until! { spi.sr.read().bsy().bit_is_set() }
        block_while! { spi.sr.read().bsy().bit_is_set() }
        let bits = spi.dr.read().bits() as u16;

        bitfield::ReadProxy::<u16, T>::new(bits)
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

        Drv8323rs {
            spi,
            mode_state: Idle {},
        }
    }
}

// pub struct RegProxy<R, const ADDR: u8> {
//     value: u16,
//     _marker: PhantomData<R>,
// }

// impl<R, const ADDR: u8> core::ops::Deref for RegProxy<R, ADDR> {
//     type Target = R;
//     #[inline(always)]
//     fn deref(&self) -> &Self::Target {
//         unsafe { self }
//     }
// }

impl Drv8323rs<Idle> {
    pub fn fault_status_1<'a>(&'a self) -> DrvRegister<'a, registers::fs1::FaultStatus1> {
        DrvRegister {
            spi: &self.spi,
            _marker: PhantomData,
        }
    }
}
