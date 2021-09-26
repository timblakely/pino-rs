use super::{ControlHardware, ControlLoop, LoopState};
use core::f32::consts::PI;
// Field-oriented control. Very basic Park/Clark forward and inverse. Currently no SVM is performed,
// and only a single i_q/i_d value is accepted S

pub struct FieldOrientedControl {
    i_q: f32,
    i_d: f32,
}

impl FieldOrientedControl {
    pub fn new(i_q: f32, i_d: f32) -> FieldOrientedControl {
        FieldOrientedControl { i_q, i_d }
    }
}

impl ControlLoop for FieldOrientedControl {
    fn commutate(&mut self, hardware: &mut ControlHardware) -> LoopState {
        let encoder = &hardware.encoder;
        let cordic = &mut hardware.cordic;

        let eangle = encoder.electrical_angle;

        let pending = cordic.cos_sin(encoder.electrical_angle);
        let [cos, sin] = pending.get_result();

        LoopState::Running
    }
}
