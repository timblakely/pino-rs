extern crate alloc;

use super::{ControlHardware, ControlLoop, LoopState};
use crate::{
    comms::{fdcan::FdcanMessage, messages::OutgoingFdcanFrame},
    current_sensing::PhaseCurrents,
};
use alloc::boxed::Box;

// Switch a single phase via PWM and measure the steady-state current for a period of time to
// calculate the phase resistance.

// Note: Since there is no current control in this loop, the PWM duty cycle is hard-coded to be
// 0.08, which at 24V and a winding resistance of 0.33R yields a peak of 7A. The windings I'm using
// are rated for 8A continuous and would very likely be able to handle much more for brief periods,
// but I ***really*** don't want to fry anything right now :)
const MAX_PWM_DUTY_CYCLE: f32 = 0.08;

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

impl OutgoingFdcanFrame for Resistance {
    fn pack(&self) -> crate::comms::fdcan::FdcanMessage {
        FdcanMessage::new(0x12, &[self.resistance.to_bits()])
    }
}

impl<'a> ControlLoop for MeasureResistance<'a> {
    fn commutate(&mut self, hardware: &mut ControlHardware) -> LoopState {
        let current_sensor = &mut hardware.current_sensor;
        current_sensor.sampling_period_fast();

        let tim1 = &hardware.tim1;

        self.current += current_sensor.sample();
        let v_bus = current_sensor.v_bus();
        self.v_bus += v_bus;

        let duty = (self.target_voltage / v_bus).min(MAX_PWM_DUTY_CYCLE);
        let ccr = (2125. * duty) as u16;

        self.pwm_duty += duty;

        match self.phase {
            Phase::A => {
                tim1.ccr1.write(|w| w.ccr1().bits(ccr));
                tim1.ccr2.write(|w| w.ccr2().bits(0));
                tim1.ccr3.write(|w| w.ccr3().bits(0));
            }
            Phase::B => {
                tim1.ccr1.write(|w| w.ccr1().bits(0));
                tim1.ccr2.write(|w| w.ccr2().bits(ccr));
                tim1.ccr3.write(|w| w.ccr3().bits(0));
            }
            Phase::C => {
                tim1.ccr1.write(|w| w.ccr1().bits(0));
                tim1.ccr2.write(|w| w.ccr2().bits(0));
                tim1.ccr3.write(|w| w.ccr3().bits(ccr));
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
