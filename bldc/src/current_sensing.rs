use stm32g4::stm32g474 as device;

// TODO(blakely): Generalize this with HAL
pub struct CurrentSensor {
    phase_a: device::ADC1,
    phase_b: device::ADC2,
    phase_c: device::ADC3,
    v_bus: device::ADC4,
    v_refint: device::ADC5,

    from_v_refint: fn(u16, u16) -> f32,
}

pub fn new(
    phase_a: device::ADC1,
    phase_b: device::ADC2,
    phase_c: device::ADC3,
    v_bus: device::ADC4,
    v_refint: device::ADC5,
) -> CurrentSensor {
    let from_v_refint = |v_refint_reading: u16, adc_reading: u16| {
        const V_REFINT_CAL: f32 = 3.0;
        // Safety: According to STM32G474 datassheet, internal voltage reference calibration value
        // is a u16 located at 0x1FFF_75AA. Calibration was performed at 3.0V
        let v_refint_cal_value = unsafe { *((0x1FFF_75AA) as *const u16) };
        // TODO(blakely): assumes 12-bit precision
        (V_REFINT_CAL as f32 * v_refint_cal_value as f32 * adc_reading as f32)
            / (v_refint_reading as f32 * 4096f32)
    };

    CurrentSensor {
        phase_a,
        phase_b,
        phase_c,
        v_bus,
        v_refint,
        from_v_refint,
    }
}

impl CurrentSensor {
    pub fn get_v_refint(&mut self) -> f32 {
        let v_refint = self.v_refint.dr.read().bits() as u16;
        (self.from_v_refint)(v_refint, v_refint)
    }
}
