use crate::ic::ma702::{AngleState, Ma702, StreamingPolling};
use core::f32::consts::PI;

const TWO_PI: f32 = 2. * PI;

pub struct Encoder {
    ma702: Ma702<StreamingPolling>,
    pole_pairs: u8,
    angle_state: AngleState,
    pub electrical_angle: f32,
    pub electrical_velocity: f32,
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
        self.electrical_angle = match angle_state.raw_angle {
            None => 0.,
            Some(a) => (a as u32 * self.pole_pairs as u32) as f32 / 4096f32,
        };
        self.electrical_angle = match self.electrical_angle {
            t if t >= PI => t - (((t + PI) / TWO_PI) as i32) as f32 * TWO_PI,
            t if t < PI => t - (((t - PI) / TWO_PI) as i32) as f32 * TWO_PI,
            t => t,
        };

        // self.electrical_velocity = angle_state.velocity * self.pole_pairs as f32;
    }
}
