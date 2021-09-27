extern crate alloc;

use crate::comms::{fdcan::FdcanMessage, messages::ExtendedFdcanFrame};

use super::{ControlHardware, ControlLoop, LoopState};
use alloc::boxed::Box;

pub struct EncoderResults {
    angle: f32,
    velocity: f32,
    e_angle: f32,
    e_velocity: f32,
    a_cos: f32,
    a_sin: f32,
}

impl EncoderResults {
    pub fn new() -> EncoderResults {
        EncoderResults {
            angle: 0.,
            velocity: 0.,
            e_angle: 0.,
            e_velocity: 0.,
            a_cos: 0.,
            a_sin: 0.,
        }
    }
}

// Simple one-loop encoder read.
pub struct ReadEncoder<'a> {
    encoder_results: EncoderResults,
    callback: Box<dyn for<'r> FnMut(&'r EncoderResults) + 'a + Send>,
}

impl<'a> ReadEncoder<'a> {
    pub fn new(callback: impl for<'r> FnMut(&'r EncoderResults) + 'a + Send) -> ReadEncoder<'a> {
        ReadEncoder {
            encoder_results: EncoderResults::new(),
            callback: Box::new(callback),
        }
    }
}

impl<'a> ControlLoop for ReadEncoder<'a> {
    fn commutate(&mut self, hardware: &mut ControlHardware) -> LoopState {
        let results = &mut self.encoder_results;

        let encoder = &hardware.encoder;
        let cordic = &mut hardware.cordic;

        let angle_state = encoder.angle_state();
        let [cos, sin] = cordic.cos_sin(angle_state.angle).get_result();

        results.angle = angle_state.angle.in_radians();
        results.velocity = angle_state.velocity.in_radians();
        results.e_angle = encoder.electrical_angle.in_radians();
        results.e_velocity = encoder.electrical_velocity.in_radians();
        results.a_cos = cos;
        results.a_sin = sin;
        LoopState::Finished
    }

    fn finished(&mut self) {
        (self.callback)(&self.encoder_results);
    }
}

pub struct ReadEncoderMsg {}

impl ExtendedFdcanFrame for ReadEncoderMsg {
    fn pack(&self) -> crate::comms::fdcan::FdcanMessage {
        panic!("Pack not supported")
    }
    fn unpack(_: &crate::comms::fdcan::FdcanMessage) -> Self {
        ReadEncoderMsg {}
    }
}

impl ExtendedFdcanFrame for EncoderResults {
    fn unpack(_: &crate::comms::fdcan::FdcanMessage) -> Self {
        panic!("Unpack not supported")
    }

    fn pack(&self) -> crate::comms::fdcan::FdcanMessage {
        FdcanMessage::new(
            0x13,
            &[
                self.angle.to_bits(),
                self.velocity.to_bits(),
                self.e_angle.to_bits(),
                self.e_velocity.to_bits(),
                self.a_cos.to_bits(),
                self.a_sin.to_bits(),
            ],
        )
    }
}
