use core::marker::PhantomData;
use num_traits::Num;

pub trait Readable {}
pub trait Writeable {}

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

pub mod macros {
    #[macro_export]
    macro_rules! readable_accessor {
        ($fn:ident, $n:ident, $size:ty, $mask:expr, $offset:expr) => {
            #[inline(always)]
            paste::paste! {
                pub fn $fn(&self) -> [<$n _R>] {
                    [<$n _R>]::new(((self.bits >> $offset) & $mask) as $size)
                }
            }
        };
    }

    #[macro_export]
    macro_rules! writable_accessor {
        ($fn:ident, $n:ident) => {
            paste::paste! {
                #[inline(always)]
                pub fn $fn(&mut self) -> [<$n _W>]  {
                    [<$n _W>] { w: self }
                }
            }
        };
    }

    #[macro_export]
    macro_rules! readable_field {
        ($n:ident, $s:ty) => {
            paste::paste! {
                #[allow(non_camel_case_types)]
                pub type [<$n _R>] = bitfield::ReadProxy<$s, $s>;
            }
        };
    }

    #[macro_export]
    macro_rules! writable_bits {
        ($n:ident, $size:ty, $mask:expr, $offset:expr) => {
            paste::paste! {
                #[allow(non_camel_case_types)]
                pub struct [<$n _W>]<'a> {
                    w: &'a mut WriteProxy,
                }
                impl<'a> [<$n _W>]<'a> {
                    #[inline(always)]
                    pub unsafe fn bits(self, value: $size) -> &'a mut WriteProxy {
                        self.w.bits = (self.w.bits & !($mask << $offset)) | (((value as u32) & $mask) << $offset);
                        self.w
                    }
                }
            }
        }
    }

    #[macro_export]
    macro_rules! writable_bits_safe {
        ($n:ident, $size:ty, $mask:expr, $offset:expr) => {
            paste::paste! {
                impl<'a> [<$n _W>]<'a> {
                    #[inline(always)]
                    pub fn set(self, value: $size) -> &'a mut WriteProxy {
                        unsafe { self.bits(value) }
                    }
                }
            }
        };
    }

    #[macro_export]
    macro_rules! writable_variant_from {
        ($n:ident, $size:ty) => {
            impl From<$n> for $size {
                #[inline(always)]
                fn from(variant: $n) -> Self {
                    variant as _
                }
            }
        };
    }

    #[macro_export]
    macro_rules! writable_variant {
        ($n:ident, $mask:expr, $offset:expr, $variant:ident) => {
            paste::paste! {
                impl<'a> [<$n _W>]<'a> {
                    #[inline(always)]
                    pub fn variant(self, variant: $variant) -> &'a mut WriteProxy {
                        unsafe { self.bits(variant.into()) }
                    }
                }
            }
        };
    }

    #[macro_export]
    macro_rules! writable_field {
        (bitsafe $n:ident, $size:ty, $mask:expr, $offset:expr) => {
            crate::writable_bits!($n, $size, $mask, $offset);
            crate::writable_bits_safe!($n, $size, $mask, $offset);
        };
        ($n:ident, $size:ty, $mask:expr, $offset:expr) => {
            crate::writable_bits!($n, $size, $mask, $offset);
        };
        ($n:ident, $size:ty, $mask:expr, $offset:expr, $variant:ident) => {
            crate::writable_bits!($n, $size, $mask, $offset);
            crate::writable_variant_from!($variant, $size);
            crate::writable_variant!($n, $mask, $offset, $variant);
        };
    }

    #[macro_export]
    macro_rules! readwrite_field {
        (bitsafe $n:ident, $size:ty, $mask:expr, $offset:expr) => {
            crate::readable_field!($n, $size);
            crate::writable_field!(bitsafe $n, $size, $mask, $offset);
        };
        ($n:ident, $size:ty, $mask:expr, $offset:expr, $variant:ident) => {
            crate::readable_field!($n, $size);
            crate::writable_field!($n, $size, $mask, $offset, $variant);
        };
        ($n:ident, $size:ty, $mask:expr, $offset:expr) => {
            crate::readable_field!($n, $size);
            crate::writable_field!($n, $size, $mask, $offset);
        };
    }
}
