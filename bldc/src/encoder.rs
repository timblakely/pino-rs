use crate::ic::ma702::{AngleState, Ma702, StreamingPolling};

pub struct Encoder {
    ma702: Ma702<StreamingPolling>,
    pole_pairs: u8,
    angle_state: AngleState,
    electrical_angle: f32,
    electrical_velocity: f32,
}

impl Encoder {
    pub fn new(ma702: Ma702<StreamingPolling>, pole_pairs: u8) -> Encoder {
        Encoder {
            ma702,
            pole_pairs,
            angle_state: AngleState::new(),
            electrical_angle: 0.,
            electrical_velocity: 0.,
        }
    }

    pub fn update(&mut self, delta_t: f32) {
        let angle_state = self.ma702.update(delta_t);
        self.angle_state = angle_state;
        self.electrical_angle = angle_state.angle * self.pole_pairs as f32;
        self.electrical_velocity = angle_state.velocity * self.pole_pairs as f32;
    }
}
