use crate::{
    comms::fdcan::{FdcanMessage, OutgoingFdcanFrame},
    cordic::Cordic,
    current_sensing::{self, CurrentSensor, PhaseCurrents},
    encoder::{Encoder, EncoderState},
    pwm::PwmOutput,
    util::seq_lock::SeqLock,
};
use core::sync::atomic::AtomicBool;
use lazy_static::lazy_static;

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

pub use controller::{ControlLoop, Controller, Commutate};

// TODO(blakely): This is probably bad form...
pub use idle_current_distribution::*;
pub use idle_current_sensor::*;
use third_party::m4vga_rs::util::spin_lock::SpinLock;

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

// TODO(blakely): Wrap the peripherals in some slightly higher-level abstractions.
pub struct CommutationState {
    pub commutator: Option<controller::Controller>,
    pub hw: ControlHardware,
}

pub static COMMUTATION_STATE: SpinLock<Option<CommutationState>> = SpinLock::new(None);
lazy_static! {
    pub static ref SENSOR_STATE: SeqLock<Option<SensorState>> = SeqLock::new(None);
}

pub static COMMUTATING: AtomicBool = AtomicBool::new(false);

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
