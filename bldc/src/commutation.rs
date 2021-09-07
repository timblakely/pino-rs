use crate::{
    current_sensing::{self, CurrentMeasurement, CurrentSensor},
    ic::ma702::{Ma702, Streaming},
};
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

pub enum LoopState {
    Running,
    Finished,
    Idle,
}

pub struct IdleCurrentSensor {
    total_counts: u32,
    loop_count: u32,
    sample: CurrentMeasurement,
}

impl IdleCurrentSensor {
    pub fn new(duration: f32) -> IdleCurrentSensor {
        IdleCurrentSensor {
            total_counts: 0,
            loop_count: 0,
            sample: CurrentMeasurement::new(),
        }
    }
}

pub trait ControlLoop: Send + Sync {
    fn commutate(&mut self, current_sensor: &CurrentSensor<current_sensing::Sampling>)
        -> LoopState;
    fn finished(&self) {}
}

impl ControlLoop for IdleCurrentSensor {
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

    fn finished(&self) {}
}
