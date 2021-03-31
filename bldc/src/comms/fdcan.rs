//! FDCAN implementation

use core::marker::PhantomData;
use core::mem::MaybeUninit;
use core::ops::Deref;

pub mod extended_filter;
pub mod rx_fifo;
pub mod standard_filter;
pub mod tx_event;
pub mod tx_fifo;

#[repr(C)]
pub struct SramBlock {
    standard_filters: [standard_filter::StandardFilter; 28usize],
    extended_filters: [extended_filter::ExtendedFilter; 8usize],
    rx_fifo0: [rx_fifo::RxFifo; 3usize],
    rx_fifo1: [rx_fifo::RxFifo; 3usize],
    tx_event_fifo: [tx_event::TxEvent; 3usize],
    tx_buffers: [tx_fifo::TxFifo; 3usize],
}

pub struct Sram {
    _marker: PhantomData<*const ()>,
}
impl Sram {
    pub const fn ptr() -> *const SramBlock {
        0x4000_A400 as *const _
    }

    fn zero_memory() {
        const N: usize = 212;
        // Safety: This... isn't really safe, but we have to zero a section of memory at an
        // arbitraty location: FDCAN1 SRAM. It goes from 0x4000_A400 to 0x4000_A750 (exclusive).
        // Convert the memory location into an array of uninitialized values.
        let buf: &mut [MaybeUninit<u32>; N] = unsafe { core::mem::transmute(Self::ptr()) };
        for slot in buf.iter_mut() {
            // Safety: Use raw pointer intentionally so we never make a reference to the underlying
            // memory - even a temporary one - to uninitialized memory.
            unsafe {
                slot.as_mut_ptr().write(0);
            }
        }
    }

    pub fn take() -> Sram {
        Self::zero_memory();
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

// pub struct Fdcan<T> {

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
