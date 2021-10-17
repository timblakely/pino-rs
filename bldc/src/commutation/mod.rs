use crate::{
    comms::{
        fdcan::{FdcanMessage, OutgoingFdcanFrame},
        messages::Message,
    },
    cordic::Cordic,
    current_sensing::{self, CurrentSensor, PhaseCurrents},
    encoder::{Encoder, EncoderState},
    pwm::PwmOutput,
    util::{interrupts::block_interrupt, seq_lock::SeqLock},
};
use core::sync::atomic::{AtomicBool, Ordering};
use lazy_static::lazy_static;
use stm32g4::stm32g474 as device;
extern crate alloc;
use alloc::boxed::Box;

pub mod calibrate_adc;
pub mod calibrate_e_zero;
pub mod idle_current_distribution;
pub mod idle_current_sensor;
pub mod interrupt;
pub mod measure_inductance;
pub mod measure_resistance;
pub mod phase_current;
pub mod pos_vel_control;
pub mod read_encoder;
pub mod torque_control;

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
    pub control_loop: Option<Box<dyn ControlLoop>>,
    pub hw: ControlHardware,
}

pub static COMMUTATION_STATE: SpinLock<Option<CommutationState>> = SpinLock::new(None);
lazy_static! {
    pub static ref SENSOR_STATE: SeqLock<Option<SensorState>> = SeqLock::new(None);
}

pub static COMMUTATING: AtomicBool = AtomicBool::new(false);

pub struct Commutator {}

impl Commutator {
    pub fn donate_hardware(hw: ControlHardware) {
        *COMMUTATION_STATE
            .try_lock()
            .expect("Lock held while trying to donate hardware") = Some(CommutationState {
            control_loop: None,
            hw,
        });
    }

    pub fn set(commutator: impl ControlLoop + 'static) {
        block_interrupt(device::interrupt::ADC1_2, &COMMUTATION_STATE, |mut vars| {
            vars.control_loop = Some(Box::new(commutator));
        });
    }

    pub fn enable_loop() {
        block_interrupt(
            device::interrupt::ADC1_2,
            &COMMUTATION_STATE,
            |mut control_vars| {
                COMMUTATING.store(true, Ordering::Relaxed);
                control_vars.hw.pwm.enable_loop();
            },
        );
    }

    pub fn is_enabled() -> bool {
        COMMUTATING.load(Ordering::Acquire)
    }

    // TODO(blakely): Don't disable the loop until currents have settled down low enough.
    pub fn disable_loop() {
        block_interrupt(
            device::interrupt::ADC1_2,
            &COMMUTATION_STATE,
            |mut control_vars| {
                COMMUTATING.store(false, Ordering::Relaxed);
                control_vars.hw.pwm.disable_loop();
            },
        );
    }
}

pub enum CommutationLoop {
    Running,
    Finished,
}

// Trait that any control loops need to implement.
pub trait ControlLoop: Send {
    fn commutate(
        &mut self,
        sensor_state: &SensorState,
        hardware: &mut ControlHardware,
    ) -> CommutationLoop;
    fn finished(&mut self);
}

impl OutgoingFdcanFrame for SensorState {
    fn pack(&self) -> crate::comms::fdcan::FdcanMessage {
        FdcanMessage::new(
            Message::SensorState,
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
