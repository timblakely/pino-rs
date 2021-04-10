//! FDCAN implementation
use crate::{block_until, block_while, driver::FdcanShared};
use core::marker::PhantomData;
use core::mem::MaybeUninit;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicBool, Ordering};
use extended_filter::{ExtendedFilterMode, ExtendedFilterType};
use ringbuffer::RingBufferWrite;
use static_assertions::const_assert;
use stm32g4::stm32g474::{self as device, fdcan::cccr::INIT_A};
use third_party::m4vga_rs::util::{spin_lock::SpinLock, sync};

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
unsafe impl Send for Sram {}
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
        // Set clock divider. This currently assumes we're running at full speed at 170MHz.
        self.peripheral.ckdiv.modify(|_, w| w.pdiv().div1());
        // Configure SDCAN timing.
        self.peripheral.nbtp.modify(|_, w| {
            // Safety: The stm32-rs package does not have an allowable range set for these fields,
            // so it's inherently unsafe to set arbitrary bits. For now these values are hard-coded
            // to known good values.
            // 1MHz
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
        // Configure SDCAN timing.
        self.peripheral.dbtp.modify(|_, w| {
            // Safety: Same as above: the stm32-rs package does not have an allowable range set for
            // these fields.
            // 5MHz
            unsafe {
                w.dbrp()
                    .bits(0)
                    .dtseg1()
                    .bits(21)
                    .dtseg2()
                    .bits(10)
                    .dsjw()
                    .bits(10)
            }
        });
        self
    }

    pub fn fifo_mode(self) -> Self {
        // FIFO mode.
        self.peripheral.txbc.modify(|_, w| w.tfqm().fifo());
        self
    }

    pub fn configure_interrupts(self) -> Self {
        // Why does ST make 0=INT1 and 1=INT0?!
        // _WHYYYYYYYYY IS INT0 MAPPED TO EINT1?!?!?!?!?!?!??!?!?!?!?!?!?!?_
        self.peripheral.ils.modify(|_, w| {
            w
                // Tx event+error notifications on INT0
                .tferr()
                .set_bit()
                // Rx event on INT1
                .rxfifo0()
                .clear_bit()
        });
        // Enable Tx and Rx events
        self.peripheral
            .ie
            .modify(|_, w| w.tefne().set_bit().rf0ne().set_bit());
        // Enable both FDCAN interrupts.
        self.peripheral
            .ile
            .modify(|_, w| w.eint0().set_bit().eint1().set_bit());
        self
    }

    pub fn start(self) -> Fdcan<Running> {
        self.peripheral.cccr.modify(|_, w| w.init().run());
        // Block until we know we're running.
        block_until! { self.peripheral.cccr.read().init() == INIT_A::RUN };
        Fdcan {
            peripheral: self.peripheral,
            sram: self.sram,
            mode_state: Running,
        }
    }
}

pub trait StandardFdcanFrame {
    fn id(&self) -> u16;
    fn pack(&self, buffer: &mut [u32; 2]) -> u8;
}
pub trait ExtendedFdcanFrame {
    fn id(&self) -> u32;
    fn pack(&self, buffer: &mut [u32; 16]) -> u8;
}

// Just for testing; do not use in regular communication.
struct DebugMessage {
    foo: u32,
    bar: f32,
    baz: u8,
    toot: &'static [u8; 3],
}
impl ExtendedFdcanFrame for DebugMessage {
    fn id(&self) -> u32 {
        0xA
    }
    fn pack(&self, buffer: &mut [u32; 16]) -> u8 {
        buffer[0] = self.foo;
        buffer[1] = self.bar.to_bits();
        buffer[2] = (self.baz as u32) << 24
            | (self.toot[2] as u32) << 16
            | (self.toot[1] as u32) << 8
            | (self.toot[0] as u32);
        3
    }
}

impl Fdcan<Running> {
    pub fn send_message(&mut self) -> &mut Self {
        let message = DebugMessage {
            foo: 123,
            bar: 77.44,
            baz: 8,
            toot: b"ASD",
        };
        match self.next_tx() {
            Some(idx) => {
                self.sram.tx_buffers[idx].assign(&message);
                self.peripheral.txbar.modify(|_, w|
                        // Safety: No enum associated with this in stm32-rs. Bit field corresponds
                        // to which tx buffer is being used.
                        unsafe { w.ar().bits(1 << idx) })
            }
            // TODO(blakely): Some actual proper error handling here...
            None => panic!("Couldn't get tx buffer"),
        };

        self
    }

    // TODO(blakely): Move to an actual TxFifo struct/impl
    fn next_tx(&self) -> Option<usize> {
        // TODO(blakely): Handle the case where we're sending too many messages at once and the FIFO
        // can't keep up.
        Some(self.peripheral.txfqs.read().tfqpi().bits() as usize)
    }

    pub fn donate(mut self) -> FdcanShared {
        FdcanShared {
            fdcan: self.peripheral,
            sram: self.sram,
        }
    }
}

pub fn fdcan1_tx_isr() {
    let fdcan = &sync::acquire_hw(&FDCANSHARE).fdcan;
    let get_idx = fdcan.txefs.read().efgi().bits();
    // Safety: Upstream: not restricted to enum or range in stm32-rs. But since we're using the
    // value retrieved from the get index it's fine.
    fdcan.txefa.modify(|_, w| unsafe { w.efai().bits(get_idx) });

    // TODO(blakely): Actually check for Tx errors
    // Ack the Tx interrupts
    fdcan.ir.modify(|_, w| w.tfe().set_bit().tefn().set_bit());
}

pub fn fdcan1_rx_isr() {
    let shared = &sync::acquire_hw(&FDCANSHARE);

    // Figure out get index
    let get_idx = shared.fdcan.rxf0s.read().f0gi().bits();
    {
        // Lock the receive buffer. Technically only used in the main thread, but good practice to
        // drop locks as soon as you can.
        let mut guard = FDCAN_RECEIVE_BUF
            .try_lock()
            .expect("FDCAN rx ISR can't lock receive buffer");
        let receive_buf = guard
            .as_mut()
            .expect("FDCAN RX ISR handled prior to populating buffer");
        let rx_buffer = &shared.sram.rx_fifo0[get_idx as usize];
        (*receive_buf).push(ReceivedMessage {
            id: rx_buffer.id(),
            data: *rx_buffer.data(),
        });
    }
    // Acknowledge the peripheral that we've read the message.
    // Safety: Upstream: not restricted to enum or range in stm32-rs. But since we're using the
    // value retrieved from the get index it's fine.
    shared
        .fdcan
        .rxf0a
        .modify(|_, w| unsafe { w.f0ai().bits(get_idx) });
    // Finally, clear the fact that we've received an RxFIFO0 interrupt
    shared.fdcan.ir.modify(|_, w| w.rf0n().set_bit());
}

#[derive(Debug)]
pub struct ReceivedMessage {
    pub id: u32,
    pub data: [u32; 16],
}

type ReceiveBuffer = ringbuffer::ConstGenericRingBuffer<ReceivedMessage, 16>;
pub static FDCAN_RECEIVE_BUF: SpinLock<Option<&'static mut ReceiveBuffer>> = SpinLock::new(None);

pub static FDCANSHARE: SpinLock<Option<FdcanShared>> = SpinLock::new(None);

fn init_buffer() -> &'static mut ReceiveBuffer {
    static TAKEN: AtomicBool = AtomicBool::new(false);

    if TAKEN.swap(true, Ordering::AcqRel) {
        panic!("RingBuffer attempted to be acquired twice");
    }
    static mut uninit_buffer: MaybeUninit<ReceiveBuffer> = MaybeUninit::uninit();

    // Safety: Conv
    let buf: &mut [MaybeUninit<u8>; core::mem::size_of::<ReceiveBuffer>()] =
        unsafe { core::mem::transmute(&mut uninit_buffer) };

    for byte in buf.iter_mut() {
        // Safety: We're using a raw pointer here so that we never create even a _temporary_
        // reference to uninitialized memory. Since this is more complicated than a simple array,
        // this is the best way to ensure that the memory is truly zero'd prior to use, the way that
        // ConstGenericRingBuffer expects.
        unsafe {
            byte.as_mut_ptr().write(0);
        }
    }

    // Safety: The entire buffer is initialized to zero, just like ConstGenericRingBuffer expects.
    unsafe { core::mem::transmute(buf) }
}

pub fn init_receive_buf() {
    *FDCAN_RECEIVE_BUF.try_lock().unwrap() = Some(init_buffer());
}
