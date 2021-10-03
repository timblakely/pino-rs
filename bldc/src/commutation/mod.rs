use crate::{
    cordic::Cordic,
    current_sensing::{self, CurrentSensor},
    encoder::Encoder,
    util::interrupts::InterruptBLock,
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

pub static CONTROL_LOOP: InterruptBLock<Option<ControlLoopVars>> =
    InterruptBLock::new(device::interrupt::ADC1_2, None);

pub struct Commutator {}

impl Commutator {
    pub fn set<'a>(commutator: impl ControlLoop + 'a) {
        let boxed: Box<dyn ControlLoop> = Box::new(commutator);
        match *CONTROL_LOOP.lock() {
            Some(ref mut v) => (*v).control_loop = unsafe { core::mem::transmute(Some(boxed)) },
            None => panic!("Loop variables not set"),
        };
    }
}

pub enum LoopState {
    Running,
    Finished,
}

// Trait that any control loops need to implement.
pub trait ControlLoop: Send {
    fn commutate(&mut self, hardware: &mut ControlHardware) -> LoopState;
    fn finished(&mut self) {}
}
