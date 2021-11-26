use super::{Commutate, ControlHardware, LoopState, SensorState};

use crate::{
    comms::fdcan::{FdcanMessage, IncomingFdcanFrame, OutgoingFdcanFrame},
    current_sensing::PhaseCurrents,
    pi_controller::PIController,
    pwm::PwmDuty,
};
#[cfg(not(feature = "host"))]
use num_traits::float::FloatCore;

// Control current for a single phase.

pub enum Phase {
    A,
    B,
    C,
}

pub struct PhaseCurrentCmd {
    pub target_current: f32,
    pub duration: f32,
    pub k: f32,
    pub ki: f32,
    pub phase: Phase,
}

impl IncomingFdcanFrame for PhaseCurrentCmd {
    fn unpack(message: FdcanMessage) -> Self {
        let buffer = message.data;
        PhaseCurrentCmd {
            duration: f32::from_bits(buffer[0]),
            target_current: f32::from_bits(buffer[1]),
            k: f32::from_bits(buffer[2]),
            ki: f32::from_bits(buffer[3]),
            phase: match buffer[4] & 0xFFu32 {
                0 => Phase::A,
                1 => Phase::B,
                _ => Phase::C,
            },
        }
    }
}

pub struct PhaseCurrent {
    total_counts: u32,
    loop_count: u32,

    target_current: f32,
    controller: PIController,

    phase: Phase,
}

impl<'a> PhaseCurrent {
    pub fn new(duration: f32, target_current: f32, phase: Phase, k: f32, ki: f32) -> PhaseCurrent {
        PhaseCurrent {
            total_counts: (40_000 as f32 * duration) as u32,
            loop_count: 0,
            target_current,
            controller: PIController::new(k, ki, 23.9),
            phase,
        }
    }
}

impl Commutate for PhaseCurrent {
    fn commutate(
        &mut self,
        _loop_state: LoopState,
        _sensor_state: &SensorState,
        hardware: &mut ControlHardware,
    ) -> LoopState {
        let current_sensor = &mut hardware.current_sensor;
        current_sensor.sampling_period_fast();

        let pwm = &mut hardware.pwm;

        let current = current_sensor.sample();
        let v_bus = current_sensor.v_bus();

        let current = match self.phase {
            Phase::A => current.phase_a,
            Phase::B => current.phase_b,
            Phase::C => current.phase_c,
        };
        // Safeguard against anything unexpected
        if current > 20. || current < -20. {
            pwm.zero_phases();
            return LoopState::Idle;
        };
        let target_voltage = self.controller.update(current, self.target_current);
        let duty = target_voltage.abs() / v_bus;

        // Note: since we want to sense _positive_ current on one phase, we actually want to
        // _switch_ the other phases so that when the ADC triggers and measures the back-EMF, the
        // current is flowing _out_ the desired phase.
        let pwms = match self.phase {
            Phase::A => PwmDuty {
                a: duty,
                b: 1. - duty,
                c: 1. - duty,
            },
            Phase::B => PwmDuty {
                a: 1. - duty,
                b: duty,
                c: 1. - duty,
            },
            Phase::C => PwmDuty {
                a: 1. - duty,
                b: 1. - duty,
                c: duty,
            },
        };
        pwm.set_pwm_duty_cycles(pwms);

        self.loop_count += 1;
        match self.loop_count {
            x if x >= self.total_counts => {
                pwm.zero_phases();
                LoopState::Idle
            }
            _ => LoopState::Running,
        }
    }

    fn finished(&mut self) {
        let _asdf = self.loop_count;
    }
}

impl OutgoingFdcanFrame for PhaseCurrents {
    fn pack(&self) -> FdcanMessage {
        FdcanMessage::new(
            0xD,
            &[
                self.phase_a.to_bits(),
                self.phase_b.to_bits(),
                self.phase_c.to_bits(),
            ],
        )
    }
}
