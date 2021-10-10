use core::marker::PhantomData;

use stm32g4::stm32g474::GPIOB;

// Badly hacked LED control :(
// TODO(blakely): TIM4 should be used for BLDCV1 if we ever want to PWM the LEDs.

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
        Self::on();
        let retval = callback();
        Self::off();
        retval
    }

    pub fn on()
    where
        Self: LedBit,
    {
        // Safety: atomic write to bit set/reset regsiter with no side effects.
        unsafe {
            (*GPIOB::ptr()).bsrr.write(|w| w.bits(1 << Self::bit()));
        }
    }

    pub fn off()
    where
        Self: LedBit,
    {
        // Safety: atomic write to bit set/reset regsiter with no side effects.
        unsafe {
            (*GPIOB::ptr())
                .bsrr
                .write(|w| w.bits(1 << (Self::bit() + 16)));
        }
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
