pub trait Readable {}
pub trait Writeable {}

use core::marker::PhantomData;
pub struct Bitfield<U, T> {
    register: vcell::VolatileCell<U>,
    _marker: PhantomData<T>,
}

impl<U, T> Bitfield<U, T>
where
    U: Copy,
{
    pub fn read(&self) -> ReadProxy<U, Self> {
        ReadProxy {
            bits: self.register.get(),
            _marker: PhantomData,
        }
    }
}

impl<U, T> Bitfield<U, T>
where
    Self: Readable + Writeable,
    U: Copy,
{
    pub fn update<F>(&self, f: F)
    where
        for<'a> F:
            FnOnce(&ReadProxy<U, Self>, &'a mut WriteProxy<U, Self>) -> &'a mut WriteProxy<U, Self>,
    {
        let bits = self.register.get();
        self.register.set(
            f(
                &ReadProxy {
                    bits,
                    _marker: PhantomData,
                },
                &mut WriteProxy {
                    bits,
                    _marker: PhantomData,
                },
            )
            .bits,
        )
    }
}

pub struct ReadProxy<U, T> {
    pub bits: U,
    _marker: PhantomData<T>,
}

impl<U, T> ReadProxy<U, T>
where
    U: Copy,
{
    #[inline(always)]
    pub fn new(bits: U) -> Self {
        Self {
            bits,
            _marker: PhantomData,
        }
    }

    #[inline(always)]
    pub fn bits(&self) -> U {
        self.bits
    }
}

impl<U, T, FI> PartialEq<FI> for ReadProxy<U, T>
where
    U: PartialEq,
    FI: Copy + Into<U>,
{
    #[inline(always)]
    fn eq(&self, other: &FI) -> bool {
        self.bits.eq(&(*other).into())
    }
}
// TODO(blakely): Specialize for bool

pub struct WriteProxy<U, T> {
    pub bits: U,
    _marker: PhantomData<T>,
}
impl<U, T> WriteProxy<U, T> {
    pub unsafe fn bits(&mut self, bits: U) -> &mut Self {
        self.bits = bits;
        self
    }
}
