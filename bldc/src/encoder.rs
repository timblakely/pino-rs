use crate::ic::ma702::{Ma702, StreamingPolling};

pub struct Encoder {
    ma702: Ma702<StreamingPolling>,
    // velocity: f32,
    // elec_velocity: f32,
}

impl Encoder {
    pub fn new(ma702: Ma702<StreamingPolling>) -> Encoder {
        Encoder { ma702 }
    }

    pub fn update(&mut self, delta_t: f32) {
        let _angle_state = self.ma702.update(delta_t);
        let mut asdf = 1;
        asdf += 1;
    }
}
