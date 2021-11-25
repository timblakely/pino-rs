use crate::{
    comms::fdcan::{FdcanMessage, OutgoingFdcanFrame},
    cordic::Cordic,
    current_sensing::{self, CurrentSensor, PhaseCurrents},
    encoder::{Encoder, EncoderState},
    pwm::PwmOutput,
};

pub mod calibrate_adc;
pub mod calibrate_e_zero;
pub mod controller;
pub mod idle_current_distribution;
pub mod idle_current_sensor;
pub mod interrupt;
pub mod measure_inductance;
pub mod measure_resistance;
pub mod phase_current;
pub mod pos_vel_control;
pub mod read_encoder;
pub mod torque_control;

pub use controller::{Commutate, Controller, LoopState};

// TODO(blakely): This is probably bad form...
pub use idle_current_distribution::*;
pub use idle_current_sensor::*;

pub struct ControlHardware {
    pub current_sensor: CurrentSensor<current_sensing::Ready>,
    pub pwm: PwmOutput,
    pub encoder: Encoder,
    pub cordic: Cordic,
}

// TODO(blakely): don't require these to be Copy/Clone; use references instead.
#[derive(Clone, Copy)]
pub struct SensorState {
    pub encoder_state: EncoderState,
    pub currents: PhaseCurrents,
    pub v_bus: f32,
}

impl SensorState {
    pub fn new(encoder_state: &EncoderState, currents: &PhaseCurrents, v_bus: f32) -> SensorState {
        SensorState {
            encoder_state: *encoder_state,
            currents: *currents,
            v_bus,
        }
    }
}

impl OutgoingFdcanFrame for SensorState {
    fn pack(&self) -> crate::comms::fdcan::FdcanMessage {
        FdcanMessage::new(
            0x1B,
            &[
                self.encoder_state.angle.in_radians().to_bits(),
                self.encoder_state.angle_multiturn.in_radians().to_bits(),
                self.encoder_state.velocity.in_radians().to_bits(),
                self.encoder_state.electrical_angle.in_radians().to_bits(),
                self.encoder_state
                    .electrical_velocity
                    .in_radians()
                    .to_bits(),
            ],
        )
    }
}
