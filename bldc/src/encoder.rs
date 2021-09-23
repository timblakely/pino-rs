use third_party::m4vga_rs::util::{armv7m::clear_pending_irq, sync::acquire_hw};

use crate::ic::ma702::{AngleState, Ma702, Streaming, ANGLE, DMA1, MA702_STATE_WRITER};
use core::f32::consts::PI;
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

const FREQ_HZ: f32 = 1000.;
const TWO_PI: f32 = 2. * PI;

// TODO(blakely): Move to ma702.
// This is the interrupt that fires when the transfer from the `SPI[DR]` register transaction is complete.
#[interrupt]
fn DMA1_CH2() {
    // Clear pending IRQ in NVIC.
    clear_pending_irq(device::Interrupt::DMA1_CH2);

    let mut state = acquire_hw(&MA702_STATE_WRITER).update();

    // Safety: We're reading a mutable static, which is usually unsafe; potentially moreso because
    // the value is written to by the DMA. However, we're inside of the DMA interrupt handler and
    // reads of this should be atomic.
    let angle_copy = unsafe { ANGLE };
    let raw_angle = unsafe { ANGLE >> 4 };
    let angle = raw_angle as f32 / 4096. * TWO_PI;
    let last_angle = match state.last_angle {
        None => state.angle,
        Some(angle) => angle,
    };
    let last_velocity = match state.last_velocity {
        None => 0.,
        Some(velocity) => velocity,
    };
    // TODO(blakely): This assumes 1kHz.
    let angular_velocity = (angle - last_angle) / FREQ_HZ;
    let angular_acceleration = (angular_velocity - last_velocity) / FREQ_HZ;

    *state = AngleState {
        raw_angle,
        angle,
        angular_velocity,
        angular_acceleration,
        last_angle: Some(last_angle),
        last_velocity: Some(last_velocity),
    };

    // Finally clear the IRQ flag in the DMA itself.
    let dma = acquire_hw(&DMA1);
    dma.ifcr.write(|w| w.gif2().set_bit());
}
