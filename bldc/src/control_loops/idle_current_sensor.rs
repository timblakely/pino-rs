use super::{LoopState, ControlHardware, Commutate, SensorState};
use crate::{
    comms::fdcan::{FdcanMessage, IncomingFdcanFrame},
    current_sensing::PhaseCurrents,
};

// Current sense for a given duration.
pub struct IdleCurrentSenseCmd {
    pub duration: f32,
}

// During commutation, no PWM is performed. The current is sampled once at each loop for a given
// duration then averaged across all samples.
pub struct IdleCurrentSensor {
    total_counts: u32,
    loop_count: u32,
    sample: PhaseCurrents,
    callback: for<'r> fn(&'r PhaseCurrents),
}

impl IdleCurrentSensor {
    pub fn new(duration: f32, callback: for<'r> fn(&'r PhaseCurrents)) -> IdleCurrentSensor {
        IdleCurrentSensor {
            total_counts: (40_000 as f32 * duration) as u32,
            loop_count: 0,
            sample: PhaseCurrents::new(),
            callback,
        }
    }
}

impl Commutate for IdleCurrentSensor {
    fn commutate(
        &mut self,
        _sensor_state: &SensorState,
        hardware: &mut ControlHardware,
    ) -> LoopState {
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

impl IncomingFdcanFrame for IdleCurrentSenseCmd {
    fn unpack(message: FdcanMessage) -> Self {
        let buffer = message.data;
        IdleCurrentSenseCmd {
            duration: f32::from_bits(buffer[0]),
        }
    }
}
