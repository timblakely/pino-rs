use crate::util::bitfield;
use crate::util::bitfield::{Bitfield, Readable, Writeable};
pub type ReadProxy = bitfield::ReadProxy<u32, StandardFilter>;
pub type WriteProxy = bitfield::WriteProxy<u32, StandardFilter>;
use paste::paste;

macro_rules! readable_field {
    ($n:ident, $s:ty) => {
        paste! {
            #[allow(non_camel_case_types)]
            pub type [<$n _R>] = bitfield::ReadProxy<$s, $s>;
        }
    };
}

macro_rules! writable_bits {
    ($n:ident, $size:ty, $mask:expr, $offset:expr) => {
        paste! {
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

macro_rules! writable_variant {
    ($n:ident, $mask:expr, $offset:expr, $variant:ident) => {
        paste! {
            impl<'a> [<$n _W>]<'a> {
                #[inline(always)]
                pub fn variant(self, variant: $variant) -> &'a mut WriteProxy {
                    unsafe { self.bits(variant.into()) }
                }
            }
        }
    };
}

macro_rules! writable_field {
    ($n:ident, $size:ty, $mask:expr, $offset:expr) => {
        writable_bits!($n, $size, $mask, $offset);
    };

    ($n:ident, $size:ty, $mask:expr, $offset:expr, $variant:ident) => {
        writable_bits!($n, $size, $mask, $offset);
        writable_variant_from!($variant, $size);
        writable_variant!($n, $mask, $offset, $variant);
    };
}
pub enum FilterType {
    Range = 0b00,
    Dual = 0b01,
    Classic = 0b10,
    Disabled = 0b11,
}
readable_field!(SFT, u8);
writable_field!(SFT, u8, 0b11, 30, FilterType);

pub enum Action {
    Disable = 0b000,
    StoreRxFIFO0 = 0b001,
    StoreRxFIFO1 = 0b010,
    Reject = 0b011,
    SetPriority = 0b100,
    SetPriorityStoreRxFIFO0 = 0b101,
    SetPriorityStoreRxFIFO1 = 0b110,
}
readable_field!(SFEC, u8);
writable_field!(SFEC, u8, 0b111, 27, Action);

readable_field!(SFID1, u16);
writable_field!(SFID1, u16, 0x7FF, 16);

readable_field!(SFID2, u16);
writable_field!(SFID2, u16, 0x7FF, 0);

#[allow(missing_docs)]
#[doc(hidden)]
pub struct _StandardFilter;

impl ReadProxy {
    #[inline(always)]
    pub fn sft(&self) -> SFT_R {
        SFT_R::new(((self.bits >> 30) & 0b11) as u8)
    }
    #[inline(always)]
    pub fn sfec(&self) -> SFEC_R {
        SFEC_R::new(((self.bits >> 27) & 0b111) as u8)
    }
    #[inline(always)]
    pub fn sfid1(&self) -> SFID1_R {
        SFID1_R::new(((self.bits >> 16) & 0x7FF) as u16)
    }
    #[inline(always)]
    pub fn sfid2(&self) -> SFID2_R {
        SFID2_R::new(((self.bits >> 0) & 0x7FF) as u16)
    }
}

impl WriteProxy {
    #[inline(always)]
    pub fn sft(&mut self) -> SFT_W {
        SFT_W { w: self }
    }
    #[inline(always)]
    pub fn sfec(&mut self) -> SFEC_W {
        SFEC_W { w: self }
    }
    #[inline(always)]
    pub fn sfid1(&mut self) -> SFID1_W {
        SFID1_W { w: self }
    }
    #[inline(always)]
    pub fn sfid2(&mut self) -> SFID2_W {
        SFID2_W { w: self }
    }
}

pub type StandardFilter = Bitfield<u32, _StandardFilter>;
impl Readable for StandardFilter {}
impl Writeable for StandardFilter {}
