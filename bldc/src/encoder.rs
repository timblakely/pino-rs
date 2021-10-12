use crate::ic::ma702::{AngleState, Ma702, StreamingPolling};
use third_party::ang::Angle;

#[derive(Clone, Copy)]
pub struct EncoderState {
    electrical_angle: Angle,
    electrical_velocity: Angle,
}

impl EncoderState {
    pub fn new(electrical_angle: Angle, electrical_velocity: Angle) -> EncoderState {
        EncoderState {
            electrical_angle,
            electrical_velocity,
        }
    }
}

pub struct Encoder {
    ma702: Ma702<StreamingPolling>,
    pole_pairs: u8,
    angle_state: AngleState,
    state: EncoderState,
}

impl Encoder {
    pub fn new(ma702: Ma702<StreamingPolling>, pole_pairs: u8) -> Encoder {
        Encoder {
            ma702,
            pole_pairs,
            angle_state: AngleState::new(),
            state: EncoderState::new(Angle::Radians(0.), Angle::Radians(0.)),
        }
    }

    pub fn update(&mut self, delta_t: f32) {
        let angle_state = self.ma702.update(delta_t);
        self.angle_state = angle_state;
        // TODO(blakely): This may be less accurate than using the conversion from raw_angle.
        let electrical_angle = (angle_state.angle * self.pole_pairs as f32).normalized();
        let electrical_velocity = (angle_state.velocity * self.pole_pairs as f32).normalized();

        self.state = EncoderState::new(electrical_angle, electrical_velocity);
    }

    pub fn electrical_angle(&self) -> Angle {
        self.state.electrical_angle
    }

    pub fn electrical_velocity(&self) -> Angle {
        self.state.electrical_velocity
    }

    pub fn state(&self) -> &EncoderState {
        &self.state
    }

    pub fn angle_state(&self) -> &AngleState {
        &self.angle_state
    }
}
