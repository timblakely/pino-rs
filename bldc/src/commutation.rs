use crate::ic::ma702::{Ma702, Streaming};
use stm32g4::stm32g474 as device;

// TODO(blakely): Wrap the peripherals in some slightly higher-level abstractions.
pub struct Hardware {
    pub tim1: device::TIM1,
    pub ma702: Ma702<Streaming>,
    pub adcs: (
        device::ADC1,
        device::ADC2,
        device::ADC3,
        device::ADC4,
        device::ADC5,
    ),
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
