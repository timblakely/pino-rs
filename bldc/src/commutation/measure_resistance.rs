extern crate alloc;

use super::{CommutationLoop, ControlHardware, ControlLoop, SensorState};

use crate::{
    comms::fdcan::{FdcanMessage, IncomingFdcanFrame, OutgoingFdcanFrame},
    current_sensing::PhaseCurrents,
    pwm::PwmDuty,
};
use alloc::boxed::Box;

// Switch a single phase via PWM and measure the steady-state current for a period of time to
// calculate the phase resistance.

// Note: Since there is no current control in this loop, the PWM duty cycle is hard-coded to be
// 0.08, which at 24V and a winding resistance of 0.33R yields a peak of 7A. The windings I'm using
// are rated for 8A continuous and would very likely be able to handle much more for brief periods,
// but I ***really*** don't want to fry anything right now :)
const MAX_PWM_DUTY_CYCLE: f32 = 0.08;

// Measure the resistance of the windings.
pub struct MeasureResistanceCmd {
    pub duration: f32,
    pub target_voltage: f32,
    pub phase: crate::commutation::measure_resistance::Phase,
}

pub enum Phase {
    A,
    B,
    C,
}

pub struct MeasureResistance<'a> {
    total_counts: u32,
    loop_count: u32,

    target_voltage: f32,

    phase: Phase,
    pwm_duty: f32,
    v_bus: f32,
    current: PhaseCurrents,

    callback: Box<dyn for<'r> FnMut(&'r Resistance) + 'a + Send>,
}

impl<'a> MeasureResistance<'a> {
    pub fn new(
        duration: f32,
        target_voltage: f32,
        phase: Phase,
        callback: impl for<'r> FnMut(&'r Resistance) + 'a + Send,
    ) -> MeasureResistance<'a> {
        MeasureResistance {
            total_counts: (40_000 as f32 * duration) as u32,
            loop_count: 0,
            target_voltage,
            phase,
            pwm_duty: 0.,
            v_bus: 0.,
            current: PhaseCurrents::new(),
            callback: Box::new(callback),
        }
    }
}

pub struct Resistance {
    resistance: f32,
}

impl<'a> ControlLoop for MeasureResistance<'a> {
    fn commutate(
        &mut self,
        _sensor_state: &SensorState,
        hardware: &mut ControlHardware,
    ) -> CommutationLoop {
        let current_sensor = &mut hardware.current_sensor;
        current_sensor.sampling_period_fast();

        self.current += current_sensor.sample();
        let v_bus = current_sensor.v_bus();
        self.v_bus += v_bus;

        let duty = (self.target_voltage / v_bus).min(MAX_PWM_DUTY_CYCLE);

        self.pwm_duty += duty;

        let pwm = &mut hardware.pwm;
        let pwms = match self.phase {
            Phase::A => PwmDuty {
                a: duty,
                b: 0.,
                c: 0.,
            },
            Phase::B => PwmDuty {
                a: 0.,
                b: duty,
                c: 0.,
            },
            Phase::C => PwmDuty {
                a: 0.,
                b: 0.,
                c: duty,
            },
        };
        pwm.set_pwm_duty_cycles(pwms);

        self.loop_count += 1;
        match self.loop_count {
            x if x >= self.total_counts => {
                pwm.zero_phases();
                CommutationLoop::Finished
            }
            _ => CommutationLoop::Running,
        }
    }

    fn finished(&mut self) {
        self.pwm_duty /= self.loop_count as f32;
        self.v_bus /= self.loop_count as f32;
        self.current /= self.loop_count;
        let resistance = (self.v_bus * self.pwm_duty)
            / match self.phase {
                Phase::A => self.current.phase_a,
                Phase::B => self.current.phase_b,
                Phase::C => self.current.phase_c,
            };
        (self.callback)(&Resistance { resistance });
    }
}

impl IncomingFdcanFrame for MeasureResistanceCmd {
    fn unpack(message: FdcanMessage) -> Self {
        let buffer = message.data;
        MeasureResistanceCmd {
            duration: f32::from_bits(buffer[0]),
            target_voltage: f32::from_bits(buffer[1]),

            phase: match buffer[2] & 0xFFu32 {
                0 => crate::commutation::measure_resistance::Phase::A,
                1 => crate::commutation::measure_resistance::Phase::B,
                _ => crate::commutation::measure_resistance::Phase::C,
            },
        }
    }
}

impl OutgoingFdcanFrame for Resistance {
    fn pack(&self) -> crate::comms::fdcan::FdcanMessage {
        FdcanMessage::new(0x12, &[self.resistance.to_bits()])
    }
}
