use super::{ControlHardware, ControlLoop, LoopState};
use crate::{
    current_sensing::PhaseCurrents,
    pi_controller::PIController,
    pwm::{PhaseVoltages, PwmDuty},
};

// Field-oriented control. Very basic Park/Clark forward and inverse. Currently no SVM is performed,
// and only a single i_q/i_d value is accepted S

const TWO_THIRDS: f32 = 0.6666666666666;
const SQRT_3: f32 = 1.73205080757;
const FRAC_SQRT_3_2: f32 = SQRT_3 / 2.;
// TODO(blakely): Hardcoded here
const DT: f32 = 1. / 40_000.;
const _MIN_PWM_VALUE: f32 = 0.;
const _MAX_PWM_VALUE: f32 = 2125.;
const PWM_INVERT: bool = true;

pub struct DQCurrents {
    pub q: f32,
    pub d: f32,
}

struct DQVoltages {
    q: f32,
    d: f32,
}

pub struct FieldOrientedControl {
    currents: DQCurrents,
    q_controller: PIController,
    d_controller: PIController,

    loops: u32,
}

impl FieldOrientedControl {
    pub fn new(currents: DQCurrents) -> FieldOrientedControl {
        // TODO(blakely): Don't hard-code these; instead pull from either global config,
        // calibration, or FDCAN command.
        FieldOrientedControl {
            currents,
            q_controller: PIController::new(1.421142407046769, 0.055681818, 24.),
            d_controller: PIController::new(1.421142407046769, 0.055681818, 24.),
            loops: 0,
        }
    }
}

fn forward_park_clark(phase_currents: PhaseCurrents, cos: f32, sin: f32) -> DQCurrents {
    let half_cos = cos / 2.;
    let half_sin = sin / 2.;

    let PhaseCurrents {
        phase_a: a,
        phase_b: b,
        phase_c: c,
        ..
    } = phase_currents;

    // Simplified DQ0 transform.
    let d = TWO_THIRDS
        * (cos * a + (FRAC_SQRT_3_2 * sin - half_cos) * b + (-FRAC_SQRT_3_2 * sin - half_cos) * c);
    let q = TWO_THIRDS
        * (-sin * a - (-FRAC_SQRT_3_2 * cos - half_sin) * b - (FRAC_SQRT_3_2 * cos - half_sin) * c);
    DQCurrents { d, q }
}

fn inverse_park_clark(dq_voltages: DQVoltages, cos: f32, sin: f32) -> PhaseVoltages {
    let half_cos = cos / 2.;
    let half_sin = sin / 2.;

    let DQVoltages { d, q } = dq_voltages;

    let a = cos * d - sin * q;
    let b = (FRAC_SQRT_3_2 * sin - half_cos) * d - (-FRAC_SQRT_3_2 * cos - half_sin) * q;
    let c = (-FRAC_SQRT_3_2 * sin - half_cos) * d - (FRAC_SQRT_3_2 * cos - half_sin) * q;
    PhaseVoltages { a, b, c }
}

fn _space_vector_modulation(v_ref: f32, phase_voltages: PhaseVoltages) -> PwmDuty {
    let PhaseVoltages {
        a: a_raw,
        b: b_raw,
        c: c_raw,
    } = phase_voltages;
    let v_min = a_raw.min(b_raw).min(c_raw);
    let v_max = a_raw.max(b_raw).max(c_raw);
    let v_offset = (v_min + v_max) / 2.;
    // const PWM_OFFSET: f32 = (0.0 + 0.94) / 2.;
    let pwm_offset: f32 = (0.0 + 0.94) / 2.;
    let a = (0.5 * (a_raw - v_offset) / v_ref + pwm_offset)
        .max(0.)
        .min(0.94);
    let b = (0.5 * (b_raw - v_offset) / v_ref + pwm_offset)
        .max(0.)
        .min(0.94);
    let c = (0.5 * (c_raw - v_offset) / v_ref + pwm_offset)
        .max(0.)
        .min(0.94);
    PwmDuty { a, b, c }
}

impl ControlLoop for FieldOrientedControl {
    fn commutate(&mut self, hardware: &mut ControlHardware) -> LoopState {
        self.loops += 1;
        let encoder = &hardware.encoder;
        let cordic = &mut hardware.cordic;
        // Kick off CORDIC conversion
        let pending_cos_sin = cordic.cos_sin(encoder.electrical_angle);

        let _asdf = encoder.electrical_angle.in_radians();
        // Sample ADCs in the meantime
        let phase_currents = hardware.current_sensor.sample();
        // Actually get the results of the Cos/Sin transform.
        let [cos, sin] = pending_cos_sin.get_result();
        // Calculate the park/clark currents
        let dq_currents = forward_park_clark(phase_currents, cos, sin);

        // Kick off new CORDIC conversion for future electrical theta
        // TODO(blakely): Why does Ben use 1.5x here?
        let new_electrical_theta =
            encoder.electrical_angle + 1.5f32 * DT * encoder.electrical_velocity;
        let pending_cos_sin = cordic.cos_sin(new_electrical_theta);
        // In the meantime, update the controllers for d and q axes
        let new_q_voltage = self.q_controller.update(dq_currents.q, self.currents.q);
        let new_d_voltage = self.d_controller.update(dq_currents.d, self.currents.d);
        // Get the result of the new theta.
        let [cos, sin] = pending_cos_sin.get_result();
        let new_voltages = inverse_park_clark(
            DQVoltages {
                q: new_q_voltage,
                d: new_d_voltage,
            },
            cos,
            sin,
        );

        // Get the current rail voltage.
        let v_bus = hardware.current_sensor.v_bus();
        let tim1 = &hardware.tim1;

        let pwms = match PWM_INVERT {
            false => PwmDuty {
                a: new_voltages.a / v_bus * 0.5 + 0.5,
                b: new_voltages.b / v_bus * 0.5 + 0.5,
                c: new_voltages.c / v_bus * 0.5 + 0.5,
            },
            true => PwmDuty {
                a: -new_voltages.a / v_bus * 0.5 + 0.5,
                b: -new_voltages.b / v_bus * 0.5 + 0.5,
                c: -new_voltages.c / v_bus * 0.5 + 0.5,
            },
        };

        // Set PWM values
        tim1.ccr1.write(|w| w.ccr1().bits((pwms.a * 2125.) as u16));
        tim1.ccr2.write(|w| w.ccr2().bits((pwms.b * 2125.) as u16));
        tim1.ccr3.write(|w| w.ccr3().bits((pwms.c * 2125.) as u16));
        LoopState::Running
    }

    fn finished(&mut self) {}
}
