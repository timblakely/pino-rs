//! FDCAN implementation

use core::marker::PhantomData;
use core::mem::MaybeUninit;
use core::ops::{Deref, DerefMut};
use extended_filter::{ExtendedFilterMode, ExtendedFilterType};
use static_assertions::const_assert;

pub mod extended_filter;
pub mod rx_fifo;
pub mod standard_filter;
pub mod tx_event;
pub mod tx_fifo;

#[repr(C)]
pub struct SramBlock {
    standard_filters: [standard_filter::StandardFilter; 28usize],
    pub extended_filters: [extended_filter::ExtendedFilter; 8usize],
    rx_fifo0: [rx_fifo::RxFifo; 3usize],
    rx_fifo1: [rx_fifo::RxFifo; 3usize],
    tx_event_fifo: [tx_event::TxEvent; 3usize],
    tx_buffers: [tx_fifo::TxFifo; 3usize],
}
// Ensure that the size of the FDCANM SRAM block is what we expect it to be.
const_assert!(core::mem::size_of::<SramBlock>() == 0x350usize);

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
impl DerefMut for Sram {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *Sram::mut_ptr() }
    }
}

pub struct Fdcan {
    pub sram: Sram,
}

impl Fdcan {
    pub fn new() -> Fdcan {
        Fdcan { sram: Sram::take() }
    }

    pub fn set_extended_filter(
        &mut self,
        i: usize,
        mode: ExtendedFilterMode,
        filter_type: ExtendedFilterType,
        id1: u32,
        id2: u32,
    ) -> &mut Self {
        let filter = &mut self.sram.extended_filters[i];
        filter
            .f0
            .update(|_, w| w.mode().variant(mode).id1().set(id1));
        filter
            .f1
            .update(|_, w| w.filter_type().variant(filter_type).id2().set(id2));
        self
    }
}
