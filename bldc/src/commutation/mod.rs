use crate::{
    current_sensing::{self, CurrentMeasurement, CurrentSensor},
    ic::ma702::{Ma702, Streaming},
    util::interrupts::InterruptBLock,
};
use stm32g4::stm32g474 as device;
extern crate alloc;
use alloc::boxed::Box;

pub mod idle_current_distribution;
pub mod idle_current_sensor;
pub mod inductance_measurement;
pub mod interrupt;

// TODO(blakely): This is probably bad form...
pub use idle_current_distribution::*;
pub use idle_current_sensor::*;

pub struct ControlHardware {
    pub current_sensor: CurrentSensor<current_sensing::Ready>,
    pub tim1: device::TIM1,
    pub ma702: Ma702<Streaming>,
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

pub struct CalibrateADC<'a> {
    total_counts: u32,
    loop_count: u32,
    sample: CurrentMeasurement,
    callback: Box<dyn for<'r> FnMut(&'r CurrentMeasurement) + 'a + Send>,
}

impl<'a> CalibrateADC<'a> {
    pub fn new(
        duration: f32,
        callback: impl for<'r> FnMut(&'r CurrentMeasurement) + 'a + Send,
    ) -> CalibrateADC<'a> {
        CalibrateADC {
            total_counts: (40_000 as f32 * duration) as u32,
            loop_count: 0,
            sample: CurrentMeasurement::new(),
            callback: Box::new(callback),
        }
    }
}

impl<'a> ControlLoop for CalibrateADC<'a> {
    fn commutate(&mut self, hardware: &mut ControlHardware) -> LoopState {
        self.loop_count += 1;
        let current_sensor = &mut hardware.current_sensor;
        self.sample += current_sensor.sample_raw();

        match self.loop_count {
            x if x >= self.total_counts => {
                self.sample /= self.loop_count;
                current_sensor.set_calibration(
                    self.sample.phase_a,
                    self.sample.phase_b,
                    self.sample.phase_c,
                );
                LoopState::Finished
            }
            _ => LoopState::Running,
        }
    }

    fn finished(&mut self) {
        (self.callback)(&self.sample);
    }
}