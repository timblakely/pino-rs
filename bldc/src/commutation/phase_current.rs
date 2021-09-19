extern crate alloc;

use super::{ControlHardware, ControlLoop, LoopState};
use crate::comms::messages::ExtendedFdcanFrame;

// Control current for a single phase.

pub enum Phase {
    A,
    B,
    C,
}

struct PIController {
    k: f32,
    ki: f32,
    ki_integral: f32,
    v_clamp: f32,
}

impl PIController {
    fn new(k: f32, ki: f32, v_clamp: f32) -> PIController {
        PIController {
            k,
            ki,
            ki_integral: 0.,
            v_clamp,
        }
    }

    fn update(&mut self, measurement: f32, target: f32) -> f32 {
        let error = target - measurement;
        let voltage = self.k * error + self.ki_integral;
        self.ki_integral += self.k * self.ki * error;
        voltage.clamp(-self.v_clamp, self.v_clamp)
    }
}

pub struct PhaseCurrentCommand {
    pub target_current: f32,
    pub duration: f32,
    pub k: f32,
    pub ki: f32,
    pub phase: Phase,
}

impl ExtendedFdcanFrame for PhaseCurrentCommand {
    fn unpack(message: &crate::comms::fdcan::FdcanMessage) -> Self {
        let buffer = message.data;
        PhaseCurrentCommand {
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

    fn pack(&self) -> crate::comms::fdcan::FdcanMessage {
        panic!("Pack not supported")
    }
}

trait BitwiseAbs {
    fn abs(&self) -> f32;
}

impl BitwiseAbs for f32 {
    fn abs(&self) -> f32 {
        f32::from_bits((*self).to_bits() & (u32::MAX >> 1))
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

impl ControlLoop for PhaseCurrent {
    fn commutate(&mut self, hardware: &mut ControlHardware) -> LoopState {
        let current_sensor = &mut hardware.current_sensor;
        current_sensor.sampling_period_fast();

        let tim1 = &hardware.tim1;

        let current = current_sensor.sample();
        let v_bus = current_sensor.v_bus();

        let current = match self.phase {
            Phase::A => current.phase_a,
            Phase::B => current.phase_b,
            Phase::C => current.phase_c,
        };
        // Safeguard against anything unexpected
        if current > 20. || current < -20. {
            tim1.ccr1.write(|w| w.ccr1().bits(0));
            tim1.ccr2.write(|w| w.ccr2().bits(0));
            tim1.ccr3.write(|w| w.ccr3().bits(0));
            return LoopState::Finished;
        };
        let target_voltage = self.controller.update(current, self.target_current);
        let duty = target_voltage.abs() / v_bus;
        let ccr = ((2125. * duty) as u16).clamp(0, 2125);

        let (active_ccr, inactive_ccr) = match target_voltage {
            x if x < 0. => (ccr, 0),
            _ => (0, ccr),
        };

        // Note: since we want to sense _positive_ current on one phase, we actually want to
        // _switch_ the other phases so that when the ADC triggers and measures the back-EMF, the
        // current is flowing _out_ the desired phase.
        match self.phase {
            Phase::A => {
                tim1.ccr1.write(|w| w.ccr1().bits(active_ccr));
                tim1.ccr2.write(|w| w.ccr2().bits(inactive_ccr));
                tim1.ccr3.write(|w| w.ccr3().bits(inactive_ccr));
            }
            Phase::B => {
                tim1.ccr1.write(|w| w.ccr1().bits(ccr));
                tim1.ccr2.write(|w| w.ccr2().bits(active_ccr));
                tim1.ccr3.write(|w| w.ccr3().bits(ccr));
            }
            Phase::C => {
                tim1.ccr1.write(|w| w.ccr1().bits(inactive_ccr));
                tim1.ccr2.write(|w| w.ccr2().bits(inactive_ccr));
                tim1.ccr3.write(|w| w.ccr3().bits(active_ccr));
            }
        }

        self.loop_count += 1;
        match self.loop_count {
            x if x >= self.total_counts => {
                tim1.ccr1.write(|w| w.ccr1().bits(0));
                tim1.ccr2.write(|w| w.ccr2().bits(0));
                tim1.ccr3.write(|w| w.ccr3().bits(0));
                LoopState::Finished
            }
            _ => LoopState::Running,
        }
    }

    fn finished(&mut self) {
        let _asdf = self.loop_count;
    }
}
