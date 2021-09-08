use crate::{
    current_sensing::{self, CurrentMeasurement, CurrentSensor},
    ic::ma702::{Ma702, Streaming},
    util::interrupts::InterruptBLock,
};
extern crate alloc;
use alloc::boxed::Box;
use stm32g4::stm32g474 as device;

// TODO(blakely): Wrap the peripherals in some slightly higher-level abstractions.
pub struct Hardware {
    pub tim1: device::TIM1,
    pub ma702: Ma702<Streaming>,
    pub current_sensor: CurrentSensor<current_sensing::Sampling>,
    // TODO(blakely): Move this into its own struct.
    pub sign: f32,
    pub square_wave_state: u32,
}

#[derive(Clone, Copy)]
pub struct ControlParameters {
    pub pwm_duty: f32,
    pub d: f32,
    pub q: f32,
}

///////

pub struct ControlLoopVars {
    pub control_loop: Option<Box<dyn ControlLoop>>,
    pub current_sensor: CurrentSensor<current_sensing::Sampling>,
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

pub trait ControlLoop: Send {
    fn commutate(&mut self, current_sensor: &CurrentSensor<current_sensing::Sampling>)
        -> LoopState;
    fn finished(&mut self) {}
}

pub struct IdleCurrentSensor<'a> {
    total_counts: u32,
    loop_count: u32,
    sample: CurrentMeasurement,
    callback: Box<dyn for<'r> FnMut(&'r CurrentMeasurement) + 'a + Send>,
}

impl<'a> IdleCurrentSensor<'a> {
    pub fn new(
        duration: f32,
        // callback: Box<dyn for<'r> FnMut(&'r CurrentMeasurement) + 'a + Send>,
        callback: impl for<'r> FnMut(&'r CurrentMeasurement) + 'a + Send,
    ) -> IdleCurrentSensor<'a> {
        IdleCurrentSensor {
            total_counts: (40_000 as f32 * duration) as u32,
            loop_count: 0,
            sample: CurrentMeasurement::new(),
            callback: Box::new(callback),
        }
    }
}

impl<'a> ControlLoop for IdleCurrentSensor<'a> {
    fn commutate(
        &mut self,
        current_sensor: &CurrentSensor<current_sensing::Sampling>,
    ) -> LoopState {
        self.loop_count += 1;
        self.sample += current_sensor.sample();
        match self.loop_count {
            x if x >= self.total_counts => LoopState::Finished,
            _ => LoopState::Running,
        }
    }

    fn finished(&mut self) {
        self.sample /= self.loop_count;
        (self.callback)(&self.sample);
    }
}
