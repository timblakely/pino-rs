use super::{CommutationLoop, ControlHardware, ControlLoop, SensorState};
use crate::{
    comms::fdcan::{FdcanMessage, IncomingFdcanFrame},
    foc::{DQCurrents, FieldOrientedControlImpl},
    led::Led,
    pi_controller::PIController,
};

// Simple torque control using FoC.

// TODO(blakely): Hardcoded here
const DT: f32 = 1. / 40_000.;
const _MIN_PWM_VALUE: f32 = 0.;
const _MAX_PWM_VALUE: f32 = 2125.;

pub struct TorqueControl {
    foc: FieldOrientedControlImpl,
    loop_count: u32,
    total_counts: u32,
}

impl TorqueControl {
    pub fn new(duration: f32, currents: DQCurrents) -> TorqueControl {
        // TODO(blakely): Don't hard-code these; instead pull from either global config,
        // calibration, or FDCAN command.
        let q_controller = PIController::new(1.421142407046769, 0.055681818, 24.);
        let d_controller = PIController::new(1.421142407046769, 0.055681818, 24.);

        let mut foc = FieldOrientedControlImpl::new(q_controller, d_controller);
        foc.q_current(currents.q);
        foc.d_current(currents.d);
        TorqueControl {
            foc,
            loop_count: 0,
            // TODO(blakely): Don't hard code this
            total_counts: (40_000 as f32 * duration) as u32,
        }
    }
}

impl ControlLoop for TorqueControl {
    fn commutate(
        &mut self,
        _sensor_state: &SensorState,
        hardware: &mut ControlHardware,
    ) -> CommutationLoop {
        Led::<crate::led::Red>::on_while(|| {
            let encoder_state = match hardware.encoder.state() {
                None => return CommutationLoop::Running,
                Some(state) => state,
            };
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

            self.loop_count += 1;
            match self.loop_count {
                x if x >= self.total_counts => {
                    self.foc.q_current(0.);
                    self.foc.d_current(0.);
                    // if dq_currents.q < 0.1 && dq_currents.d < 0.1 {
                    //     tim1.ccr1.write(|w| w.ccr1().bits(0));
                    //     tim1.ccr2.write(|w| w.ccr2().bits(0));
                    //     tim1.ccr3.write(|w| w.ccr3().bits(0));
                    //     return LoopState::Finished;
                    // }
                    CommutationLoop::Running
                }
                _ => CommutationLoop::Running,
            }
        })
    }

    fn finished(&mut self) {}
}

pub struct TorqueControlCmd {
    pub duration: f32,
    pub currents: DQCurrents,
}

impl IncomingFdcanFrame for TorqueControlCmd {
    fn unpack(message: FdcanMessage) -> Self {
        let buffer = message.data;
        TorqueControlCmd {
            duration: f32::from_bits(buffer[0]),
            currents: DQCurrents {
                q: f32::from_bits(buffer[1]),
                d: f32::from_bits(buffer[2]),
            },
        }
    }
}
