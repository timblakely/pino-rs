use stm32g4::stm32g474 as device;

// TODO(blakely): Wrap the peripherals in some slightly higher-level abstractions.
pub struct Board {
    pub tim1: device::TIM1,
}

#[derive(Clone, Copy)]
pub struct ControlParameters {
    pub pwm_duty: f32,
    pub d: f32,
    pub q: f32,
}
