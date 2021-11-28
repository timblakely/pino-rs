use third_party::m4vga_rs::util::spin_lock::SpinLock;

use crate::{
    foc::FieldOrientedControlImpl,
    pi_controller::PIController,
    util::buffered_state::{BufferedState, StateReader, StateWriter},
};

use super::{Commutate, LoopState, SensorState};

const GEAR_RATIO: f32 = 6.0;
const DT: f32 = 1. / 40_000.;

// Position and velocity control using FoC wrapped in torque control.

static COMMAND_BUFFER: SpinLock<Option<BufferedState<PosVelState>>> = SpinLock::new(None);
static COMMAND: SpinLock<Option<StateWriter<PosVelState>>> = SpinLock::new(None);

#[derive(Clone, Copy)]
pub struct PosVelState {
    pub position: f32,
    pub velocity: f32,
    pub stiffness_gain: f32,
    pub damping_gain: f32,
    pub torque_constant: f32,
}

pub struct PositionVelocity {
    foc: FieldOrientedControlImpl,
    commands: StateReader<PosVelState>,
}

impl PositionVelocity {
    pub fn new() -> PositionVelocity {
        // TODO(blakely): Don't hard-code these; instead pull from either global config,
        // calibration, or FDCAN command.
        let q_controller = PIController::new(1.421142407046769, 0.055681818, 24.);
        let d_controller = PIController::new(1.421142407046769, 0.055681818, 24.);
        let foc = FieldOrientedControlImpl::new(q_controller, d_controller);

        let mut command_buffer = COMMAND_BUFFER.lock();
        *command_buffer = Some(BufferedState::new(PosVelState {
            position: 0.,
            velocity: 0.,
            stiffness_gain: 0.,
            damping_gain: 0.,
            torque_constant: 1.,
        }));

        let (reader, writer) = command_buffer
            .as_mut()
            .expect("No command buffer to split")
            .split();

        *COMMAND.lock() = Some(writer);

        PositionVelocity {
            foc,
            commands: reader,
        }
    }

    pub fn command<'a>(command: PosVelState) {
        if let Some(state) = &mut *COMMAND.try_lock().expect("Lock held when writing command") {
            *state.update() = command;
        }
    }
}

impl Commutate for PositionVelocity {
    fn commutate(
        &mut self,
        loop_state: LoopState,
        _sensor_state: &SensorState,
        hardware: &mut super::ControlHardware,
    ) -> LoopState {
        let encoder_state = match hardware.encoder.state() {
            None => return LoopState::Running,
            Some(state) => state,
        };
        let mech_angle = encoder_state.angle_multiturn.in_radians();
        let mech_velocity = encoder_state.velocity.in_radians();

        let commands = self.commands.read();

        let torque_desired = match loop_state {
            LoopState::Shutdown => 0.,
            _ => {
                commands.stiffness_gain * (commands.position - mech_angle)
                    + commands.damping_gain * (commands.velocity - mech_velocity)
            }
        };
        let q_current = torque_desired / (commands.torque_constant * GEAR_RATIO);
        self.foc.q_current(q_current);

        // Get the current rail voltage.
        let v_bus = hardware.current_sensor.v_bus();
        // Calculate the required PWM values via field oriented control.
        let phase_voltages = self.foc.update(
            &hardware.current_sensor,
            &encoder_state,
            &mut hardware.cordic,
            DT,
        );
        hardware.pwm.set_voltages(v_bus, phase_voltages);
        // If we're shutting down, wait until the mechanical angle is zero before we indicate we're
        // idle.
        match loop_state {
            LoopState::Shutdown => match mech_velocity {
                x if x < 0.01 => LoopState::Idle,
                _ => LoopState::Shutdown,
            },
            x => x,
        }
    }
    fn finished(&mut self) {}
}