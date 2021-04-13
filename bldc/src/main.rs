#![no_std]
#![no_main]

use bldc::driver;
use ringbuffer::RingBufferRead;
use stm32g4::stm32g474::{self as device, interrupt};
use third_party::m4vga_rs::util::armv7m::clear_pending_irq;

#[cfg(feature = "panic-halt")]
use panic_halt as _;
#[cfg(feature = "panic-itm")]
use panic_itm as _; // you can put a breakpoint on `rust_begin_unwind` to catch panics

// TODO(blakely): Comment on all the stuff that happens before we actually get
// here...
#[cortex_m_rt::entry]
fn main() -> ! {
    let controller = driver::take_hardware().configure_peripherals();

    loop {
        // Not only do we lock the receive buffer, but we prevent the FDCAN_INTR1 from firing - the
        // only other interrupt that shares this particular buffer - so ensure we aren't preempted
        // when reading from it. This is fine in general since the peripheral itself has an internal
        // buffer, and as long as we can clear the backlog before the peripheral receives 4 requests
        // we shoudl be good.
        // Alternatively, we could just process a single message here to make sure that we only hold
        // this lock for the absolute minimum time.
        bldc::util::interrupts::free_from(
            device::interrupt::FDCAN1_INTR1_IT,
            &FDCAN_RECEIVE_BUF,
            |mut buf| {
                while let Some(message) = buf.dequeue_ref() {
                    let _asdf = message;
                }
            },
        );
        let angle = controller.mode_state.ma702.angle();
        let _asdf = angle;
    }
}

use bldc::comms::fdcan::FDCAN_RECEIVE_BUF;

#[interrupt]
fn FDCAN1_INTR0_IT() {
    bldc::comms::fdcan::fdcan1_tx_isr();
    clear_pending_irq(device::Interrupt::FDCAN1_INTR0_IT);
}

#[interrupt]
fn FDCAN1_INTR1_IT() {
    bldc::comms::fdcan::fdcan1_rx_isr();
    clear_pending_irq(device::Interrupt::FDCAN1_INTR1_IT);
}
