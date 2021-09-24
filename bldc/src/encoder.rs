use crate::ic::ma702::{Ma702, Streaming};

pub struct Encoder {
    ma702: Ma702<Streaming>,
    // velocity: f32,
    // elec_velocity: f32,
}

impl Encoder {
    pub fn new(ma702: Ma702<Streaming>) -> Encoder {
        Encoder { ma702 }
    }

    pub fn update(&mut self, delta_t: f32) {}
}
