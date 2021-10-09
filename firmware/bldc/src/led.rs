use core::marker::PhantomData;

use stm32g4::stm32g474::GPIOB;

// Badly hacked LED control :(

pub struct Red {}
pub struct Green {}
pub struct Blue {}
pub struct Led<T> {
    _marker: PhantomData<T>,
}

pub trait LedBit {
    fn bit() -> usize;
}

impl<T> Led<T> {
    pub fn on_while<R, C>(mut callback: C) -> R
    where
        C: Sized,
        C: FnMut() -> R,
        Self: LedBit,
    {
        // Safety: atomic write to bit set/reset regsiter with no side effects.
        unsafe {
            (*GPIOB::ptr()).bsrr.write(|w| w.bits(1 << Self::bit()));
        }
        let retval = callback();
        // Safety: atomic write to bit set/reset regsiter with no side effects.
        unsafe {
            (*GPIOB::ptr())
                .bsrr
                .write(|w| w.bits(1 << (Self::bit() + 16)));
        }
        retval
    }
}

impl LedBit for Led<Red> {
    fn bit() -> usize {
        6
    }
}

impl LedBit for Led<Green> {
    fn bit() -> usize {
        7
    }
}

impl LedBit for Led<Blue> {
    fn bit() -> usize {
        9
    }
}
