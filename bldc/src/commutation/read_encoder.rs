extern crate alloc;

use crate::comms::fdcan::{FdcanMessage, IncomingFdcanFrame, OutgoingFdcanFrame};
use crate::comms::messages::Message;

use super::{CommutationLoop, ControlHardware, ControlLoop, SensorState};
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
    fn commutate(
        &mut self,
        _sensor_state: &SensorState,
        hardware: &mut ControlHardware,
    ) -> CommutationLoop {
        let results = &mut self.encoder_results;

        let encoder = &hardware.encoder;
        let cordic = &mut hardware.cordic;

        if let Some(state) = encoder.state() {
            let [cos, sin] = cordic.cos_sin(state.angle).get_result();

            results.angle = state.angle.in_radians();
            results.velocity = state.velocity.in_radians();
            results.e_angle = state.electrical_angle.in_radians();
            results.e_velocity = state.electrical_velocity.in_radians();
            results.a_cos = cos;
            results.a_sin = sin;
        }
        CommutationLoop::Finished
    }

    fn finished(&mut self) {
        (self.callback)(&self.encoder_results);
    }
}

pub struct ReadEncoderMsg {}

impl IncomingFdcanFrame for ReadEncoderMsg {
    fn unpack(_: FdcanMessage) -> Self {
        ReadEncoderMsg {}
    }
}

impl OutgoingFdcanFrame for EncoderResults {
    fn pack(&self) -> FdcanMessage {
        FdcanMessage::new(
            Message::EncoderResults,
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
