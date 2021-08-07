#![no_std]
#![no_main]

use bldc::comms::fdcan::ExtendedFdcanFrame;
use bldc::{
    comms::messages::{Debug, Debug2},
    driver,
};
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
    let mut controller = driver::take_hardware().configure_peripherals();

    // controller.run(|id, buffer| match id {
    //     _ => (),
    // });

    controller.run2(|message| {
        let comms_message = match message.id {
            0xA => Some(Debug::unpack(message)),
            _ => None,
        };
        None
    });
}

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

#[interrupt]
fn ADC1_2() {
    // Main control loop.
    unsafe {
        *(0x4800_0418 as *mut u32) = 1 << 9;
        cortex_m::asm::nop();
        cortex_m::asm::nop();
        cortex_m::asm::nop();
        cortex_m::asm::nop();
        cortex_m::asm::nop();
        cortex_m::asm::nop();
        cortex_m::asm::nop();
        cortex_m::asm::nop();
        cortex_m::asm::nop();
        cortex_m::asm::nop();
        *(0x4800_0418 as *mut u32) = 1 << (9 + 16);
    }
    // HACK HACK HACK: Clear EOS for ADC 1
    unsafe {
        *(0x5000_0000 as *mut u32) = 1 << 3;
    }
    // TODO(blakely): actually do any semblance of control :P
    clear_pending_irq(device::Interrupt::ADC1_2);
}
