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
    pub fn on_while(callback: fn())
    where
        Self: LedBit,
    {
        unsafe {
            *(0x4800_0418 as *mut u32) = 1 << Self::bit();
        }
        callback();
        unsafe {
            *(0x4800_0418 as *mut u32) = 1 << (Self::bit() + 16);
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
