use crate::ic::ma702::{AngleState, Ma702, StreamingPolling};
use core::f32::consts::PI;
use third_party::ang::Angle;

const TWO_PI: f32 = 2. * PI;

pub struct Encoder {
    ma702: Ma702<StreamingPolling>,
    pole_pairs: u8,
    angle_state: AngleState,
    pub electrical_angle: Angle,
    pub electrical_velocity: Angle,
}

impl Encoder {
    pub fn new(ma702: Ma702<StreamingPolling>, pole_pairs: u8) -> Encoder {
        Encoder {
            ma702,
            pole_pairs,
            angle_state: AngleState::new(),
            electrical_angle: Angle::Radians(0.),
            electrical_velocity: Angle::Radians(0.),
        }
    }

    pub fn update(&mut self, delta_t: f32) {
        let angle_state = self.ma702.update(delta_t);
        self.angle_state = angle_state;
        // TODO(blakely): This may be less accurate than using the conversion from raw_angle.
        self.electrical_angle = (angle_state.angle * 12.).normalized();
    }
}
