#![no_std]
#![no_main]

use bldc::{block_while, driver};
use ringbuffer::RingBufferRead;
use stm32g4::stm32g474::{self as device, interrupt};
use third_party::m4vga_rs::util::armv7m::clear_pending_irq;

#[cfg(feature = "panic-halt")]
use panic_halt as _;
#[cfg(feature = "panic-itm")]
use panic_itm as _; // you can put a breakpoint on `rust_begin_unwind` to catch panics

// TODO(blakely): Comment on all the stuff happens before we actually get
// here...
#[cortex_m_rt::entry]
fn main() -> ! {
    let controller = driver::take_hardware().configure_peripherals();
    let spi = &controller.mode_state.spi1;
    let foo: *mut u16 = 0x4001300c as *mut u16;
    spi.dr.write(|w| w.dr().bits(0b1010001111000101));
    spi.cr1.modify(|_, w| w.spe().set_bit());

    spi.dr.write(|w| w.dr().bits(0b1010001111000101));
    // block_while! { spi.sr.read().bsy().bit_is_set() }
    // spi.cr1.modify(|_, w| w.spe().clear_bit());

    // systick.set_clock_source(cortex_m::peripheral::syst::SystClkSource::Core);
    // systick.set_reload(170000);
    // systick.clear_current();
    // systick.enable_counter();
    // loop {
    //     let mut toot = 0;
    //     while toot < 500 {
    //         while !systick.has_wrapped() {
    //             // loop until it's wrapped
    //         }
    //         toot += 1;
    //     }
    //     foo.gpioa.bsrr.write(|w| w.bs5().set_bit());
    //     while toot < 1000 {
    //         while !systick.has_wrapped() {
    //             // loop until it's wrapped
    //         }
    //         toot += 1;
    //     }
    //     foo.gpioa.bsrr.write(|w| w.br5().set_bit());
    //     // iprintln!(stim, "Second tick");
    // }

    // let asdf = &FDCAN1_INTR0_IT;

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
        )
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
