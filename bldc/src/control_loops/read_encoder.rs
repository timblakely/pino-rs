use crate::comms::fdcan::{FdcanMessage, IncomingFdcanFrame, OutgoingFdcanFrame};

use super::{LoopState, ControlHardware, Commutate, SensorState};

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
pub struct ReadEncoder {
    encoder_results: EncoderResults,
    callback: for<'r> fn(&'r EncoderResults),
}

impl ReadEncoder {
    pub fn new(callback: for<'r> fn(&'r EncoderResults)) -> ReadEncoder {
        ReadEncoder {
            encoder_results: EncoderResults::new(),
            callback,
        }
    }
}

impl Commutate for ReadEncoder {
    fn commutate(
        &mut self,
        _sensor_state: &SensorState,
        hardware: &mut ControlHardware,
    ) -> LoopState {
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
        LoopState::Finished
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
