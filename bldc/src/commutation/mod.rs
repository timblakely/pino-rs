use crate::{
    cordic::Cordic,
    current_sensing::{self, CurrentSensor},
    encoder::Encoder,
    util::interrupts::block_interrupt,
};
use stm32g4::stm32g474 as device;
extern crate alloc;
use alloc::boxed::Box;

pub mod calibrate_adc;
pub mod calibrate_e_zero;
pub mod field_oriented_control;
pub mod idle_current_distribution;
pub mod idle_current_sensor;
pub mod interrupt;
pub mod measure_inductance;
pub mod measure_resistance;
pub mod phase_current;
pub mod read_encoder;

// TODO(blakely): This is probably bad form...
pub use idle_current_distribution::*;
pub use idle_current_sensor::*;
use third_party::m4vga_rs::util::spin_lock::SpinLock;

pub struct ControlHardware {
    pub current_sensor: CurrentSensor<current_sensing::Ready>,
    pub tim1: device::TIM1,
    pub encoder: Encoder,
    pub cordic: Cordic,
}

// TODO(blakely): Wrap the peripherals in some slightly higher-level abstractions.
pub struct ControlLoopVars {
    pub control_loop: Option<Box<dyn ControlLoop>>,
    pub hw: ControlHardware,
}

pub static CONTROL_LOOP: SpinLock<Option<ControlLoopVars>> = SpinLock::new(None);

pub struct Commutator {}

impl Commutator {
    pub fn donate_hardware(hw: ControlHardware) {
        *CONTROL_LOOP
            .try_lock()
            .expect("Lock held while trying to donate hardware") = Some(ControlLoopVars {
            control_loop: None,
            hw,
        });
    }

    pub fn set<'a>(commutator: impl ControlLoop + 'a) {
        block_interrupt(device::interrupt::ADC1_2, &CONTROL_LOOP, |mut vars| {
            let boxed: Box<dyn ControlLoop> = Box::new(commutator);
            vars.control_loop = unsafe { core::mem::transmute(Some(boxed)) };
        });
    }

    pub fn enable_loop() {
        block_interrupt(device::interrupt::ADC1_2, &CONTROL_LOOP, |control_vars| {
            let hw = &control_vars.hw;
            // Kick off the timer.
            hw.tim1.cr1.modify(|_, w| w.cen().set_bit());
            // Now that the timer has started, enable the main output to allow current on the pins. If
            // we do this before we enable the timer, we have the potential to get into a state where the
            // PWM pins are in an active state but the timer isn't running, potentially drawing tons of
            // current through any high phases to any low phases.
            hw.tim1.bdtr.modify(|_, w| w.moe().set_bit());
        });
    }

    pub fn disable_loop() {
        block_interrupt(device::interrupt::ADC1_2, &CONTROL_LOOP, |control_vars| {
            let hw = &control_vars.hw;
            // Disable main output.
            hw.tim1.bdtr.modify(|_, w| w.moe().clear_bit());
            // Disable the timer completely.
            hw.tim1.cr1.modify(|_, w| w.cen().clear_bit());
        });
    }
}

pub enum LoopState {
    Running,
    Finished,
}

// Trait that any control loops need to implement.
pub trait ControlLoop: Send {
    fn commutate(&mut self, hardware: &mut ControlHardware) -> LoopState;
    fn finished(&mut self);
}
