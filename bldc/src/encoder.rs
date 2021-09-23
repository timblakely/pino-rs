use third_party::m4vga_rs::util::{armv7m::clear_pending_irq, sync::acquire_hw};

use crate::ic::ma702::{Ma702, Streaming, DMA1};
use stm32g4::stm32g474::{self as device, interrupt};

pub struct Encoder {
    ma702: Ma702<Streaming>,
    // velocity: f32,
    // elec_velocity: f32,
}

impl Encoder {
    pub fn new(ma702: Ma702<Streaming>) -> Encoder {
        Encoder { ma702 }
    }

    pub fn update(&mut self, delta_t: f32) {}
}

// This is the interrupt that fires when the transfer from the `SPI[DR]` register transaction is complete.
#[interrupt]
fn DMA1_CH2() {
    // Clear pending IRQ in NVIC.
    clear_pending_irq(device::Interrupt::DMA1_CH2);
    
    // Finally clear the IRQ flag in the DMA itself.
    let dma = acquire_hw(&DMA1);
    dma.ifcr.write(|w| w.gif2().set_bit());
}
