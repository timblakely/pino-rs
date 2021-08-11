use stm32g4::stm32g474 as device;

// TODO(blakely): Wrap the peripherals in some slightly higher-level abstractions.
pub struct Board {
    pub tim1: device::TIM1,
}

pub struct ControlLoop {
    // board: Board,
    pub asdf: u32,
}
