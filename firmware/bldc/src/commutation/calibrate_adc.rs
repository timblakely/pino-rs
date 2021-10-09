extern crate alloc;
use alloc::boxed::Box;

use crate::{
    comms::fdcan::{FdcanMessage, IncomingFdcanFrame},
    current_sensing::PhaseCurrents,
};

use super::{CommutationLoop, ControlHardware, ControlLoop, SensorState};

// Calibrate ADC values.
pub struct CalibrateADCCmd {
    pub duration: f32,
}

pub struct CalibrateADC<'a> {
    total_counts: u32,
    loop_count: u32,
    sample: PhaseCurrents,
    callback: Box<dyn for<'r> FnMut(&'r PhaseCurrents) + 'a + Send>,
}

impl<'a> CalibrateADC<'a> {
    pub fn new(
        duration: f32,
        callback: impl for<'r> FnMut(&'r PhaseCurrents) + 'a + Send,
    ) -> CalibrateADC<'a> {
        CalibrateADC {
            total_counts: (40_000 as f32 * duration) as u32,
            loop_count: 0,
            sample: PhaseCurrents::new(),
            callback: Box::new(callback),
        }
    }
}

impl<'a> ControlLoop for CalibrateADC<'a> {
    fn commutate(
        &mut self,
        _sensor_state: &SensorState,
        hardware: &mut ControlHardware,
    ) -> CommutationLoop {
        self.loop_count += 1;
        let current_sensor = &mut hardware.current_sensor;
        self.sample += current_sensor.sample_raw();

        match self.loop_count {
            x if x >= self.total_counts => {
                self.sample /= self.loop_count;
                current_sensor.set_calibration(
                    self.sample.phase_a,
                    self.sample.phase_b,
                    self.sample.phase_c,
                );
                CommutationLoop::Finished
            }
            _ => CommutationLoop::Running,
        }
    }

    fn finished(&mut self) {
        (self.callback)(&self.sample);
    }
}

impl IncomingFdcanFrame for CalibrateADCCmd {
    fn unpack(message: FdcanMessage) -> Self {
        let buffer = message.data;
        CalibrateADCCmd {
            duration: f32::from_bits(buffer[0]),
        }
    }
}
