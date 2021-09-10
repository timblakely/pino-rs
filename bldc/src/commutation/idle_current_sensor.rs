extern crate alloc;

use super::{ControlHardware, ControlLoop, LoopState};
use crate::current_sensing::CurrentMeasurement;
use alloc::boxed::Box;

// During commutation, no PWM is performed. The current is sampled once at each loop for a given
// duration then averaged across all samples.
pub struct IdleCurrentSensor<'a> {
    total_counts: u32,
    loop_count: u32,
    sample: CurrentMeasurement,
    callback: Box<dyn for<'r> FnMut(&'r CurrentMeasurement) + 'a + Send>,
}

impl<'a> IdleCurrentSensor<'a> {
    pub fn new(
        duration: f32,
        callback: impl for<'r> FnMut(&'r CurrentMeasurement) + 'a + Send,
    ) -> IdleCurrentSensor<'a> {
        IdleCurrentSensor {
            total_counts: (40_000 as f32 * duration) as u32,
            loop_count: 0,
            sample: CurrentMeasurement::new(),
            callback: Box::new(callback),
        }
    }
}

impl<'a> ControlLoop for IdleCurrentSensor<'a> {
    fn commutate(&mut self, hardware: &mut ControlHardware) -> LoopState {
        self.loop_count += 1;
        let current_sensor = &hardware.current_sensor;
        self.sample += current_sensor.sample();

        match self.loop_count {
            x if x >= self.total_counts => LoopState::Finished,
            _ => LoopState::Running,
        }
    }

    fn finished(&mut self) {
        self.sample /= self.loop_count;
        (self.callback)(&self.sample);
    }
}
