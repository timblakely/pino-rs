use super::{ControlHardware, ControlLoop, LoopState};
use crate::{current_sensing::PhaseCurrents, pi_controller::PIController};

// Field-oriented control. Very basic Park/Clark forward and inverse. Currently no SVM is performed,
// and only a single i_q/i_d value is accepted S

const TWO_THIRDS: f32 = 0.6666666666666;
const SQRT_3: f32 = 1.73205080757;
const FRAC_SQRT_3_2: f32 = SQRT_3 / 2.;

struct DQCurrents {
    q: f32,
    d: f32,
}

pub struct FieldOrientedControl {
    q_current: f32,
    q_controller: PIController,
    d_current: f32,
    d_controller: PIController,
}

impl FieldOrientedControl {
    pub fn new(q_current: f32, d_current: f32) -> FieldOrientedControl {
        // TODO(blakely): Don't hard-code these; instead pull from either global config,
        // calibration, or FDCAN command.
        FieldOrientedControl {
            q_current,
            q_controller: PIController::new(1.421142407046769, 0.055681818, 24.),
            d_current,
            d_controller: PIController::new(1.421142407046769, 0.055681818, 24.),
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

impl ControlLoop for FieldOrientedControl {
    fn commutate(&mut self, hardware: &mut ControlHardware) -> LoopState {
        let encoder = &hardware.encoder;
        let cordic = &mut hardware.cordic;
        // Kick off CORDIC conversion
        let pending = cordic.cos_sin(encoder.electrical_angle);
        // Sample ADCs in the meantime
        let phase_currents = hardware.current_sensor.sample();
        // Actually get the results of the Cos/Sin transform.
        let [cos, sin] = pending.get_result();
        // Calculate the park/clark currents
        let dq_currents = forward_park_clark(phase_currents, cos, sin);

        LoopState::Running
    }
}
