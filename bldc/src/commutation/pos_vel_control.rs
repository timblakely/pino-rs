use third_party::m4vga_rs::util::spin_lock::SpinLock;

use crate::{
    comms::fdcan::{FdcanMessage, IncomingFdcanFrame},
    foc::FieldOrientedControlImpl,
    pi_controller::PIController,
    util::buffered_state::{BufferedState, StateReader, StateWriter},
};

use super::{CommutationLoop, ControlLoop, SensorState};

const GEAR_RATIO: f32 = 6.0;
const DT: f32 = 1. / 40_000.;

// Position and velocity control using FoC wrapped in torque control.

#[derive(Clone, Copy)]
pub struct PosVelCommand {
    pub position: f32,
    pub velocity: f32,
    pub stiffness_gain: f32,
    pub damping_gain: f32,
    pub torque_constant: f32,
}

impl IncomingFdcanFrame for PosVelCommand {
    fn unpack(message: FdcanMessage) -> Self {
        let buffer = message.data;
        PosVelCommand {
            position: f32::from_bits(buffer[0]),
            velocity: f32::from_bits(buffer[1]),
            stiffness_gain: f32::from_bits(buffer[2]),
            damping_gain: f32::from_bits(buffer[3]),
            torque_constant: f32::from_bits(buffer[4]),
        }
    }
}

pub struct PosVelMode {}
impl IncomingFdcanFrame for PosVelMode {
    fn unpack(_: FdcanMessage) -> Self {
        PosVelMode {}
    }
}

static COMMAND_BUFFER: SpinLock<Option<BufferedState<PosVelCommand>>> = SpinLock::new(None);
static COMMAND: SpinLock<Option<StateWriter<PosVelCommand>>> = SpinLock::new(None);

pub struct PosVelControl {
    foc: FieldOrientedControlImpl,

    commands: StateReader<PosVelCommand>,
}

impl PosVelControl {
    pub fn new() -> PosVelControl {
        // TODO(blakely): Don't hard-code these; instead pull from either global config,
        // calibration, or FDCAN command.
        let q_controller = PIController::new(1.421142407046769, 0.055681818, 24.);
        let d_controller = PIController::new(1.421142407046769, 0.055681818, 24.);
        let foc = FieldOrientedControlImpl::new(q_controller, d_controller);

        let mut command_buffer = COMMAND_BUFFER.lock();
        *command_buffer = Some(BufferedState::new(PosVelCommand {
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

        PosVelControl {
            foc,
            commands: reader,
        }
    }

    pub fn command<'a>(command: PosVelCommand) {
        if let Some(state) = &mut *COMMAND.try_lock().expect("Lock held when writing command") {
            *state.update() = command;
        }
    }
}

impl ControlLoop for PosVelControl {
    fn commutate(
        &mut self,
        _sensor_state: &SensorState,
        hardware: &mut super::ControlHardware,
    ) -> CommutationLoop {
        let mech_angle = hardware.encoder.angle_state().angle_multiturn.in_radians();
        let mech_velocity = hardware.encoder.angle_state().velocity.in_radians();

        let commands = self.commands.read();

        let torque_desired = commands.stiffness_gain * (commands.position - mech_angle)
            + commands.damping_gain * (commands.velocity - mech_velocity);

        let q_current = torque_desired / (commands.torque_constant * GEAR_RATIO);
        self.foc.q_current(q_current);

        // Get the current rail voltage.
        let v_bus = hardware.current_sensor.v_bus();
        // Calculate the required PWM values via field oriented control.
        let phase_voltages = self.foc.update(
            &hardware.current_sensor,
            &hardware.encoder,
            &mut hardware.cordic,
            DT,
        );
        hardware.pwm.set_voltages(v_bus, phase_voltages);
        CommutationLoop::Running
    }
    fn finished(&mut self) {}
}
