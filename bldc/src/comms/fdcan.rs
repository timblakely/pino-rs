//! FDCAN implementation
use crate::util::interrupts::block_interrupts;
use crate::{block_until, block_while};
use core::marker::PhantomData;
use core::mem::MaybeUninit;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicBool, Ordering};

use extended_filter::{ExtendedFilterMode, ExtendedFilterType};
use ringbuffer::RingBufferRead;
use ringbuffer::RingBufferWrite;
use static_assertions::const_assert;
use stm32g4::stm32g474::{self as device, interrupt};
use third_party::m4vga_rs::util::armv7m::clear_pending_irq;
use third_party::m4vga_rs::util::{spin_lock::SpinLock, sync};

pub mod extended_filter;
pub mod rx_fifo;
pub mod standard_filter;
pub mod tx_event;
pub mod tx_fifo;

const RECEIVE_BUFFER_SIZE: usize = 16;

type ReceiveBuffer = ringbuffer::ConstGenericRingBuffer<FdcanMessage, RECEIVE_BUFFER_SIZE>;
pub static FDCAN_RECEIVE_BUF: SpinLock<Option<&'static mut ReceiveBuffer>> = SpinLock::new(None);
static SHARED_DEVICE: SpinLock<Option<FdcanDevice>> = SpinLock::new(None);
pub const FDCAN_INTERRUPTS: [device::Interrupt; 2] = [
    device::interrupt::FDCAN1_INTR0_IT,
    device::interrupt::FDCAN1_INTR1_IT,
];

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

struct FdcanDevice {
    sram: Sram,
    fdcan: device::FDCAN1,
}

pub struct Init {
    sram: Sram,
    fdcan: device::FDCAN1,
}
pub struct Running;

pub struct Fdcan<S> {
    mode_state: S,
}

pub fn take<'a>(fdcan: device::FDCAN1) -> Fdcan<Init> {
    *FDCAN_RECEIVE_BUF.try_lock().unwrap() = Some(init_buffer());
    // Enter init mode.
    fdcan.cccr.modify(|_, w| w.init().init());
    // Block until we know we're in init mode.
    block_while! { fdcan.cccr.read().init().is_run() };
    // Enable config writing
    fdcan.cccr.modify(|_, w| w.cce().readwrite());
    Fdcan {
        mode_state: Init {
            sram: Sram::get(),
            fdcan,
        },
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
        let filter = &mut self.mode_state.sram.extended_filters[i];
        filter
            .f0
            .update(|_, w| w.mode().variant(mode).id1().set(id1));
        filter
            .f1
            .update(|_, w| w.filter_type().variant(filter_type).id2().set(id2));
        self
    }

    pub fn configure_protocol(self) -> Self {
        self.mode_state.fdcan.cccr.modify(|_, w| {
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
        self.mode_state.fdcan.ckdiv.modify(|_, w| w.pdiv().div1());
        // Configure SDCAN timing.
        self.mode_state.fdcan.nbtp.modify(|_, w| {
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
        self.mode_state.fdcan.dbtp.modify(|_, w| {
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
        self.mode_state.fdcan.txbc.modify(|_, w| w.tfqm().fifo());
        self
    }

    pub fn configure_interrupts(self) -> Self {
        // Why does ST make 0=INT1 and 1=INT0?!
        // _WHYYYYYYYYY IS INT0 MAPPED TO EINT1?!?!?!?!?!?!??!?!?!?!?!?!?!?_
        self.mode_state.fdcan.ils.modify(|_, w| {
            w
                // Tx event+error notifications on INT0
                .tferr()
                .set_bit()
                // Rx event on INT1
                .rxfifo0()
                .clear_bit()
        });
        self
    }

    pub fn enable_interrupts(&self) -> &Self {
        // Enable Tx and Rx events
        self.mode_state
            .fdcan
            .ie
            .modify(|_, w| w.tefne().set_bit().rf0ne().set_bit());
        // Enable both FDCAN interrupts.
        self.mode_state
            .fdcan
            .ile
            .modify(|_, w| w.eint0().set_bit().eint1().set_bit());
        self
    }

    pub fn disable_interrupts(self) -> Self {
        self.mode_state
            .fdcan
            .ie
            .modify(|_, w| w.tefne().clear_bit().rf0ne().clear_bit());
        self.mode_state
            .fdcan
            .ile
            .modify(|_, w| w.eint0().clear_bit().eint1().clear_bit());
        self
    }

    pub fn start(self) -> Fdcan<Running> {
        // Enable interrupts.
        self.enable_interrupts();
        let Init { fdcan, sram } = self.mode_state;
        // Donate the device and SRAM to the interrupts,
        *SHARED_DEVICE.lock() = Some(FdcanDevice { fdcan, sram });
        // We needx access to the resources we just donated to enable the device, so we block the
        // interrupts while we start to make sure the device is fully ready.
        block_interrupts(FDCAN_INTERRUPTS, &SHARED_DEVICE, |shared| {
            // Enter run mode.
            shared.fdcan.cccr.modify(|_, w| w.init().run());
            // Block until we know we're running.
            block_until! { shared.fdcan.cccr.read().init().is_run() };
        });
        Fdcan {
            mode_state: Running,
        }
    }
}

pub fn send_message(message: &impl OutgoingFdcanFrame) {
    send_serialized_message(message.pack());
}

fn send_serialized_message(message: FdcanMessage) {
    // Block interrupts, acquiring the shared hardware.
    block_interrupts(FDCAN_INTERRUPTS, &SHARED_DEVICE, |mut shared| {
        // TODO(blakely): Move to an actual TxFifo struct/impl
        let tx_idx = shared.fdcan.txfqs.read().tfqpi().bits() as usize;
        shared.sram.tx_buffers[tx_idx].assign(&message);
        // Safety: No enum associated with this in stm32-rs. Bit field corresponds
        // to which tx buffer is being used.
        shared
            .fdcan
            .txbar
            .modify(|_, w| unsafe { w.ar().bits(1 << tx_idx) });
    });
}

impl Fdcan<Running> {
    pub fn pending_message(&self) -> Option<FdcanMessage> {
        // Not only do we lock the receive buffer, but we prevent the FDCAN_INTR1 (Rx) from
        // firing - the only other interrupt that shares this particular buffer - ensuring
        // we aren't preempted when reading from it. This is fine in general since the
        // peripheral itself has an internal buffer, but we don't want to block the interrupt for
        // too long in case there's a torrent of incoming messages.
        crate::util::interrupts::block_interrupts(
            FDCAN_INTERRUPTS,
            &FDCAN_RECEIVE_BUF,
            |mut buf| buf.dequeue(),
        )
    }
}

fn fdcan1_tx_isr() {
    let fdcan = &sync::acquire_hw(&SHARED_DEVICE).fdcan;
    let get_idx = fdcan.txefs.read().efgi().bits();
    // Safety: Upstream: not restricted to enum or range in stm32-rs. But since we're using the
    // value retrieved from the get index it's fine.
    fdcan.txefa.modify(|_, w| unsafe { w.efai().bits(get_idx) });

    // TODO(blakely): Actually check for Tx errors
    // Ack the Tx interrupts
    fdcan.ir.modify(|_, w| w.tfe().set_bit().tefn().set_bit());
}

fn fdcan1_rx_isr() {
    let shared = &sync::acquire_hw(&SHARED_DEVICE);

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
        (*receive_buf).push(FdcanMessage {
            id: rx_buffer.id(),
            data: *rx_buffer.data(),
            size: rx_buffer.len(),
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

#[derive(Debug, Clone)]
pub struct FdcanMessage {
    pub id: u32,
    pub data: [u32; 16],
    pub size: u8,
}

impl FdcanMessage {
    pub fn new<const T: usize>(message: u32, data: &[u32; T]) -> FdcanMessage {
        // There's no real idiomatic way to zero-initialize-and-fill-in-up-to-length in Rust as of
        // Aug '21.
        let mut message = FdcanMessage {
            id: message,
            data: [0; 16],
            size: T as u8,
        };
        let len = T.min(16);
        message.data[..len].copy_from_slice(&data[..len]);
        message
    }
}

fn init_buffer() -> &'static mut ReceiveBuffer {
    static TAKEN: AtomicBool = AtomicBool::new(false);

    if TAKEN.swap(true, Ordering::AcqRel) {
        panic!("RingBuffer attempted to be acquired twice");
    }
    static mut UNINIT_BUFFER: MaybeUninit<ReceiveBuffer> = MaybeUninit::uninit();
    // Safety: we're effectively writing zeros to an arbitrary place in memory, but
    // `MaybeUninit::uninit` reserves enough space for a `ReceiveBuffer`, and
    // `MaybeUninit::zeroed()` knows exactly how many bytes need to be zeroed.
    unsafe { UNINIT_BUFFER = MaybeUninit::zeroed() };
    // Safety: Now that the buffer is zeroed, we derefernce the `MaybeUninit<ReceiveBuffer>` - which
    // is unsafe - but since it's statically allocated it's alright to return its address since it's
    // guaranteed not to change.
    unsafe { &mut *UNINIT_BUFFER.as_mut_ptr() }
}

pub trait IncomingFdcanFrame {
    // Unpack the message from a buffer.
    fn unpack(message: FdcanMessage) -> Self;
}

pub trait OutgoingFdcanFrame {
    // Pack the message into a buffer of up to 64 bytes, returning the number of bytes that were
    // packed.
    fn pack(&self) -> FdcanMessage;
}

#[interrupt]
fn FDCAN1_INTR0_IT() {
    clear_pending_irq(device::Interrupt::FDCAN1_INTR0_IT);
    fdcan1_tx_isr();
}

#[interrupt]
fn FDCAN1_INTR1_IT() {
    clear_pending_irq(device::Interrupt::FDCAN1_INTR1_IT);
    fdcan1_rx_isr();
}
