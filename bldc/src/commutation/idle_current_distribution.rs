extern crate alloc;

use super::{ControlHardware, ControlLoop, LoopState};
use alloc::boxed::Box;

// Sample current one one phase for a period of time, building a histogram of currents.

enum Phase {
    A,
    B,
    C,
}

pub struct IdleCurrentDistribution<'a> {
    total_counts: u32,
    loop_count: u32,
    bins: [u32; 16],
    current_min: f32,
    current_binsize: f32,
    phase: Phase,
    callback: Box<dyn for<'r> FnMut(&'r [u32; 16]) + 'a + Send>,
}

impl<'a> IdleCurrentDistribution<'a> {
    pub fn new(
        duration: f32,
        center: f32,
        range: f32,
        phase: u8,
        callback: impl for<'r> FnMut(&'r [u32; 16]) + 'a + Send,
    ) -> IdleCurrentDistribution<'a> {
        let current_min = center - range;
        let current_binsize = range * 2. / 16.;
        let phase = match phase {
            0 => Phase::A,
            1 => Phase::B,
            _ => Phase::C,
        };
        IdleCurrentDistribution {
            total_counts: (40_000 as f32 * duration) as u32,
            loop_count: 0,
            bins: [0; 16],
            current_min,
            current_binsize,
            phase,
            callback: Box::new(callback),
        }
    }
}

impl<'a> ControlLoop for IdleCurrentDistribution<'a> {
    fn commutate(&mut self, hardware: &mut ControlHardware) -> LoopState {
        self.loop_count += 1;
        let current_sensor = &hardware.current_sensor;
        let sample = current_sensor.sample();

        let current_value = match self.phase {
            Phase::A => sample.phase_a,
            Phase::B => sample.phase_b,
            Phase::C => sample.phase_c,
        };

        let bin_index = ((current_value - self.current_min) / self.current_binsize) as usize;
        let bin_index = bin_index.max(0).min(self.bins.len() - 1);
        self.bins[bin_index] += 1;

        match self.loop_count {
            x if x >= self.total_counts => LoopState::Finished,
            _ => LoopState::Running,
        }
    }

    fn finished(&mut self) {
        (self.callback)(&self.bins);
    }
}
