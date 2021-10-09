use crate::{
    cordic::Cordic,
    current_sensing::{CurrentSensor, PhaseCurrents, Ready},
    encoder::Encoder,
    pi_controller::PIController,
    pwm::PhaseVoltages,
};

// Field-oriented control. Very basic Park/Clark forward and inverse. Currently no SVM is performed,
// and only a single i_q/i_d value is accepted.

const TWO_THIRDS: f32 = 0.6666666666666;
const SQRT_3: f32 = 1.73205080757;
const FRAC_SQRT_3_2: f32 = SQRT_3 / 2.;

pub struct PhaseDuty {
    pub a: f32,
    pub b: f32,
    pub c: f32,
}

pub struct DQCurrents {
    pub q: f32,
    pub d: f32,
}

struct DQVoltages {
    q: f32,
    d: f32,
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

// fn _space_vector_modulation(v_ref: f32, phase_voltages: PhaseVoltages) -> PhaseDuty {
//     let PhaseVoltages {
//         a: a_raw,
//         b: b_raw,
//         c: c_raw,
//     } = phase_voltages;
//     let v_min = a_raw.min(b_raw).min(c_raw);
//     let v_max = a_raw.max(b_raw).max(c_raw);
//     let v_offset = (v_min + v_max) / 2.;
//     // const PWM_OFFSET: f32 = (0.0 + 0.94) / 2.;
//     let pwm_offset: f32 = (0.0 + 0.94) / 2.;
//     let a = (0.5 * (a_raw - v_offset) / v_ref + pwm_offset)
//         .max(0.)
//         .min(0.94);
//     let b = (0.5 * (b_raw - v_offset) / v_ref + pwm_offset)
//         .max(0.)
//         .min(0.94);
//     let c = (0.5 * (c_raw - v_offset) / v_ref + pwm_offset)
//         .max(0.)
//         .min(0.94);
//     PhaseDuty { a, b, c }
// }

pub struct FieldOrientedControlImpl {
    q_controller: PIController,
    d_controller: PIController,

    q_current_target: f32,
    d_current_target: f32,
}

impl FieldOrientedControlImpl {
    pub fn new(q_controller: PIController, d_controller: PIController) -> FieldOrientedControlImpl {
        FieldOrientedControlImpl {
            q_controller,
            d_controller,
            q_current_target: 0.,
            d_current_target: 0.,
        }
    }

    pub fn q_current(&mut self, current: f32) {
        self.q_current_target = current;
    }

    pub fn d_current(&mut self, current: f32) {
        self.d_current_target = current;
    }

    pub fn update(
        &mut self,
        current_sensor: &CurrentSensor<Ready>,
        encoder: &Encoder,
        cordic: &mut Cordic,
        dt: f32,
    ) -> PhaseVoltages {
        // Kick off CORDIC conversion
        let pending_cos_sin = cordic.cos_sin(encoder.electrical_angle());
        // Sample ADCs in the meantime
        let phase_currents = current_sensor.sample();
        // Actually get the results of the Cos/Sin transform.
        let [cos, sin] = pending_cos_sin.get_result();
        // Calculate the park/clark currents
        let dq_currents = forward_park_clark(phase_currents, cos, sin);

        // Kick off new CORDIC conversion for future electrical theta
        // TODO(blakely): Why does Ben use 1.5x here?
        let new_electrical_theta =
            encoder.electrical_angle() + 1.5f32 * dt * encoder.electrical_velocity();
        let pending_cos_sin = cordic.cos_sin(new_electrical_theta);
        // In the meantime, update the controllers for d and q axes
        let new_q_voltage = self
            .q_controller
            .update(dq_currents.q, self.q_current_target);
        let new_d_voltage = self
            .d_controller
            .update(dq_currents.d, self.d_current_target);
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
        new_voltages
    }
}
