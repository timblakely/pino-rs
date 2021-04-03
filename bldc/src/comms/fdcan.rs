//! FDCAN implementation
use crate::block_while;
use core::marker::PhantomData;
use core::mem::MaybeUninit;
use core::ops::{Deref, DerefMut};
use extended_filter::{ExtendedFilterMode, ExtendedFilterType};
use static_assertions::const_assert;
use stm32g4::stm32g474::{self as device, fdcan::cccr::INIT_A};

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

    pub fn get() -> Sram {
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

pub struct Uninit;
pub struct Init;
pub struct Running;

pub trait EnterInit {}
impl EnterInit for Uninit {}
impl EnterInit for Running {}

pub struct Fdcan<S> {
    sram: Sram,
    peripheral: device::FDCAN1,
    #[allow(dead_code)]
    mode_state: S,
}

pub fn take(fdcan: device::FDCAN1) -> Fdcan<Uninit> {
    Fdcan {
        sram: Sram::get(),
        peripheral: fdcan,
        mode_state: Uninit {},
    }
}

impl<S: EnterInit> Fdcan<S> {
    pub fn enter_init(self) -> Fdcan<Init> {
        self.peripheral.cccr.modify(|_, w| w.init().init());
        // Block until we know we're in init mode.
        block_while! { self.peripheral.cccr.read().init() == INIT_A::RUN };
        // Enable config writing
        self.peripheral.cccr.modify(|_, w| w.cce().readwrite());
        Fdcan {
            sram: self.sram,
            peripheral: self.peripheral,
            mode_state: Init {},
        }
    }
}

impl Fdcan<Init> {
    pub fn set_extended_filter(
        mut self,
        i: usize,
        mode: ExtendedFilterMode,
        filter_type: ExtendedFilterType,
        id1: u32,
        id2: u32,
    ) -> Self {
        let filter = &mut self.sram.extended_filters[i];
        filter
            .f0
            .update(|_, w| w.mode().variant(mode).id1().set(id1));
        filter
            .f1
            .update(|_, w| w.filter_type().variant(filter_type).id2().set(id2));
        self
    }

    pub fn configure_protocol(self) -> Self {
        self.peripheral.cccr.modify(|_, w| {
            w // Enable TX pause
                .txp()
                .clear_bit()
                // No edge filtering
                .efbi()
                .clear_bit()
                // Protocol exception handling disabled.
                .pxhd()
                .clear_bit()
                // Enable bit rate switching
                .brse()
                .set_bit()
                // Enable FD
                .fdoe()
                .set_bit()
                // No test mode
                .test()
                .normal()
                // Enable automatic retransmission
                .dar()
                .retransmit()
                // No bus monitoring
                .mon()
                .clear_bit()
                // No restricted mode.
                .asm()
                .normal()
                // No sleep mode
                .csr()
                .clear_bit()
        });
        self
    }

    pub fn configure_timing(self) -> Self {
        self.peripheral.nbtp.modify(|_, w| {
            // Safety: The stm32-rs package does not have an allowable range set for these fields,
            // so it's inherently unsafe to set arbitrary bits. For now these values are hard-coded
            // to known good values.
            unsafe {
                w.nbrp()
                    .bits(4)
                    .ntseg1()
                    .bits(21)
                    .ntseg2()
                    .bits(10)
                    .nsjw()
                    .bits(5)
            }
        });
        self.peripheral.dbtp.modify(|_, w| {
            // Safety: Same as above: the stm32-rs package does not have an allowable range set for
            // these fields.
            unsafe {
                w.dbrp()
                    .bits(0)
                    .dtseg1()
                    .bits(32)
                    .dtseg2()
                    .bits(10)
                    .dsjw()
                    .bits(10)
            }
        });
        self
    }

    pub fn start(self) -> Fdcan<Running> {
        Fdcan {
            peripheral: self.peripheral,
            sram: self.sram,
            mode_state: Running,
        }
    }
}
