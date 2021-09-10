extern crate alloc;

use super::{ControlHardware, ControlLoop, LoopState};
use crate::current_sensing::CurrentMeasurement;
use alloc::boxed::Box;

// Drive a zero-centered square wave through the phases, which should result in a triangle wave of
// current through the inductor. Measuring current over time should give us the inductance via
// V=L(di/dt). This appoximation holds as long as the square wave duration is ⋘ inductor time
// constant τ.

// Note: Since there is no current control in this loop, the PWM duty cycle is hard-coded to be
// 0.03, which at 24V and a winding resistance of 0.2R yields 3.6A. The windings I'm using are rated
// for 8A continuous and would very likely be able to handle much more, but I don't want to fry
// anything right now :)

enum Direction {
    Up,
    Down,
}

pub struct InductanceMeasurement<'a> {
    total_counts: u32,
    loop_count: u32,
    direction: Direction,
    sample: CurrentMeasurement,
    switch_count: u32,
    loops_per_switch: f32,
    remainder: f32,
    last_sample: Option<CurrentMeasurement>,
    callback: Box<dyn FnMut(f32) + 'a + Send>,
}

impl<'a> InductanceMeasurement<'a> {
    pub fn new(
        duration: f32,
        frequency: u32,
        callback: impl FnMut(f32) + 'a + Send,
    ) -> InductanceMeasurement<'a> {
        InductanceMeasurement {
            total_counts: (40_000 as f32 * duration) as u32,
            loop_count: 0,
            direction: Direction::Up,
            sample: CurrentMeasurement::new(),
            loops_per_switch: (40_000 as f32) / frequency as f32,
            switch_count: 0,
            remainder: 0.,
            last_sample: None,
            callback: Box::new(callback),
        }
    }
}

impl<'a> ControlLoop for InductanceMeasurement<'a> {
    fn commutate(&mut self, hardware: &mut ControlHardware) -> LoopState {
        let current_sensor = &hardware.current_sensor;
        let sample = current_sensor.sample();

        let sign: f32 = match self.direction {
            Direction::Up => 1.,
            Direction::Down => -1.,
        };

        self.sample += match self.last_sample {
            None => &sample * sign,
            Some(ref last) => (&sample - last) * sign,
        };
        self.last_sample = Some(sample);

        self.switch_count += 1;
        let count_and_remainder: f32 = self.switch_count as f32 + self.remainder;
        if count_and_remainder >= self.loops_per_switch {
            self.remainder = count_and_remainder - self.loops_per_switch;
            self.direction = match self.direction {
                Direction::Up => {
                    hardware
                        .tim1
                        .ccr1
                        .write(|w| w.ccr1().bits((0.03f32 * 2125.) as u16));
                    hardware.tim1.ccr2.write(|w| w.ccr2().bits(0));
                    hardware.tim1.ccr3.write(|w| w.ccr3().bits(0));
                    Direction::Down
                }
                Direction::Down => {
                    hardware.tim1.ccr1.write(|w| w.ccr1().bits(0));
                    hardware
                        .tim1
                        .ccr2
                        .write(|w| w.ccr2().bits((0.03f32 * 2125.) as u16));
                    hardware
                        .tim1
                        .ccr3
                        .write(|w| w.ccr3().bits((0.03f32 * 2125.) as u16));
                    Direction::Up
                }
            };
        }

        self.loop_count += 1;
        match self.loop_count {
            x if x >= self.total_counts => LoopState::Finished,
            _ => LoopState::Running,
        }
    }

    fn finished(&mut self) {
        let inductance = self.sample.v_bus * 0.03 / self.loop_count as f32;
        (self.callback)(inductance);
    }
}
