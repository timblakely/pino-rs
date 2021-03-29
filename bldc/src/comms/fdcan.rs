//! FDCAN implementation

use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};

mod bitfield {
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
        pub fn modify<F>(&self, f: F)
        where
            for<'a> F: FnOnce(
                &ReadProxy<U, Self>,
                &'a mut WriteProxy<U, Self>,
            ) -> &'a mut WriteProxy<U, Self>,
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
}

pub mod standard_filter {
    use core::fmt::Write;

    use super::bitfield;
    use super::bitfield::{Bitfield, Readable, Writeable};
    pub type ReadProxy = bitfield::ReadProxy<u32, StandardFilter>;
    pub type WriteProxy = bitfield::WriteProxy<u32, StandardFilter>;

    pub type StandardFilter = Bitfield<u32, _StandardFilter>;
    impl Readable for StandardFilter {}
    impl Writeable for StandardFilter {}

    pub type SFT_R = bitfield::ReadProxy<u8, u8>;
    pub struct SFT_W<'a> {
        w: &'a mut WriteProxy,
    }
    pub enum StandardFilterType {
        Range = 0b00,
        Dual = 0b01,
        Classic = 0b10,
        Disabled = 0b11,
    }
    impl From<StandardFilterType> for u8 {
        #[inline(always)]
        fn from(variant: StandardFilterType) -> Self {
            variant as _
        }
    }
    impl<'a> SFT_W<'a> {
        #[inline(always)]
        pub unsafe fn bits(self, value: u8) -> &'a mut WriteProxy {
            self.w.bits = (self.w.bits & !(0b11 << 30)) | (((value as u32) & 0b11) << 20);
            self.w
        }
        #[inline(always)]
        pub fn variant(self, variant: StandardFilterType) -> &'a mut WriteProxy {
            unsafe { self.bits(variant.into()) }
        }
        #[inline(always)]
        pub fn classic(self) -> &'a mut WriteProxy {
            self.variant(StandardFilterType::Classic)
        }
    }

    pub type SFEC_R = bitfield::ReadProxy<u8, u8>;
    pub type SFID1_R = bitfield::ReadProxy<u16, u16>;
    pub type SFID2_R = bitfield::ReadProxy<u16, u16>;

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
            SFID1_R::new(((self.bits >> 16) & 0b11111111111) as u16)
        }
        #[inline(always)]
        pub fn sfid2(&self) -> SFID2_R {
            SFID2_R::new(((self.bits >> 0) & 0b11111111111) as u16)
        }
    }

    impl WriteProxy {
        #[inline(always)]
        pub fn sft(&mut self) -> SFT_W {
            SFT_W { w: self }
        }

        #[inline(always)]
        pub fn classic(&mut self) -> &mut WriteProxy {
            SFT_W { w: self }.variant(StandardFilterType::Classic)
        }
    }
}

#[repr(C)]
pub struct SramBlock {
    pub standard_filters: [standard_filter::StandardFilter; 28usize],
    // extended_filters: [ExtendedFilter; 8usize],
    // rx_fifo0: [RxFifo; 3usize],
    // rx_fifo1: [RxFifo; 3usize],
    // tx_event_fifo: [TxEvent; 3usize],
    // tx_buffers: [TxBuffer; 3usize],
}

pub struct Sram {
    _marker: PhantomData<*const ()>,
}
impl Sram {
    pub const fn ptr() -> *const SramBlock {
        0x4000_A400 as *const _
    }

    pub const fn mut_ptr() -> *mut SramBlock {
        0x4000_A400 as *mut _
    }

    pub fn take() -> Sram {
        Sram {
            _marker: PhantomData,
        }
    }
}
impl Deref for Sram {
    type Target = SramBlock;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        unsafe { &*Sram::ptr() }
    }
}
// TODO(blakely): Remove once write proxies are implemented.
impl DerefMut for Sram {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *Sram::mut_ptr() }
    }
}

// pub enum ExtendedFilterMode {
//     Disable = 0b000,
//     StoreRxFIFO0 = 0b001,
//     StoreRxFIFO1 = 0b010,
//     Reject = 0b011,
//     SetPriority = 0b100,
//     SetPriorityStoreRxFIFO0 = 0b101,
//     SetPriorityStoreRxFIFO1 = 0b110,
// }

// pub enum ExtendedFilterType {
//     Range = 0b00,
//     Dual = 0b01,
//     Classic = 0b10,
//     RangeNoXIDAM = 0b11,
// }

// pub struct ExtendedMessageFilter {
//     f0: u32,
//     f1: u32,
// }

// impl ExtendedMessageFilter {
//     pub fn set(
//         &mut self,
//         mode: ExtendedFilterMode,
//         filter: ExtendedFilterType,
//         id1: u32,
//         id2: u32,
//     ) {
//         self.f0 = ((mode as u32) << 29) | (id1 & !(0b111 << 29));
//         self.f1 = ((filter as u32) << 30) | (id2 & !(0b11 << 30));
//     }

//     pub fn clear(&mut self) {
//         self.f0 = 0;
//         self.f1 = 0;
//     }
// }

// pub struct ExtendedMessageFilterBlock {
//     filters: [ExtendedMessageFilter; 8],
// }

// impl ExtendedMessageFilterBlock {
//     pub fn filter(&mut self, i: usize) -> &mut ExtendedMessageFilter {
//         &mut self.filters[i]
//     }
// }

// pub struct ExtendedMessageFilterMem {
//     _marker: PhantomData<*const ()>,
// }
// // unsafe impl Send for I2C4 {}
// impl ExtendedMessageFilterMem {
//     ///Returns a pointer to the register block
//     #[inline(always)]
//     pub const fn ptr() -> *const ExtendedMessageFilterBlock {
//         0x4000_A400 as *const _
//     }
// }
// impl Deref for ExtendedMessageFilterMem {
//     type Target = ExtendedMessageFilterBlock;
//     #[inline(always)]
//     fn deref(&self) -> &Self::Target {
//         unsafe { &*ExtendedMessageFilterMem::ptr() }
//     }
// }
// impl DerefMut for ExtendedMessageFilterMem {
//     #[inline(always)]
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         unsafe { &mut *ExtendedMessageFilterMem::ptr() }
//     }
// }

// pub struct Fdcan {
//     pub extended_filters: ExtendedMessageFilterMem,
// }

// impl Fdcan {
//     pub fn new() -> Fdcan {
//         Fdcan {
//             extended_filters: ExtendedMessageFilterMem {
//                 _marker: PhantomData,
//             },
//         }
//     }
// }
