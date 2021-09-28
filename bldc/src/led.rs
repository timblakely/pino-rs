use core::marker::PhantomData;

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
        unsafe {
            *(0x4800_0418 as *mut u32) = 1 << Self::bit();
        }
        let retval = callback();
        unsafe {
            *(0x4800_0418 as *mut u32) = 1 << (Self::bit() + 16);
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
