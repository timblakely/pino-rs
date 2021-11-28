use super::{Commutate, ControlHardware, LoopState, SensorState};
use crate::comms::messages::EZeroMsg;
use crate::{
    foc::{DQCurrents, FieldOrientedControlImpl},
    led::Led,
    pi_controller::PIController,
};

// Field-oriented control. Very basic Park/Clark forward and inverse. Currently no SVM is performed,
// and only a single i_q/i_d value is accepted S

// TODO(blakely): Hardcoded here
const DT: f32 = 1. / 40_000.;
const _MIN_PWM_VALUE: f32 = 0.;
const _MAX_PWM_VALUE: f32 = 2125.;

pub struct CalibrateEZero {
    foc: FieldOrientedControlImpl,

    total_counts: u32,
    loop_count: u32,

    record: EZeroMsg,

    callback: for<'r> fn(&'r EZeroMsg),
}

impl CalibrateEZero {
    pub fn new(
        duration: f32,
        currents: DQCurrents,
        callback: for<'r> fn(&'r EZeroMsg),
    ) -> CalibrateEZero {
        let q_controller = PIController::new(1.421142407046769, 0.055681818, 24.);
        let d_controller = PIController::new(1.421142407046769, 0.055681818, 24.);

        let mut foc = FieldOrientedControlImpl::new(q_controller, d_controller);
        foc.q_current(currents.q);
        foc.d_current(currents.d);

        // TODO(blakely): Don't hard-code these; instead pull from either global config,
        // calibration, or FDCAN command.
        CalibrateEZero {
            foc,
            total_counts: (40_000 as f32 * duration) as u32,
            loop_count: 0,
            callback,

            record: EZeroMsg {
                angle: 0.,
                angle_raw: 0,
                e_angle: 0.,
                e_raw: 0.,
            },
        }
    }
}

impl Commutate for CalibrateEZero {
    fn commutate(
        &mut self,
        _loop_state: LoopState,
        sensor_state: &SensorState,
        hardware: &mut ControlHardware,
    ) -> LoopState {
        Led::<crate::led::Red>::on_while(|| {
            let ControlHardware {
                ref current_sensor,
                ref encoder,
                ref mut cordic,
                ..
            } = hardware;
            let encoder_state = match encoder.state() {
                None => return LoopState::Running,
                Some(state) => state,
            };
            // Calculate the required PWM values via field oriented control.
            let phase_voltages = self.foc.update(current_sensor, encoder_state, cordic, DT);
            hardware
                .pwm
                .set_voltages(sensor_state.v_bus, phase_voltages);

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
                    LoopState::Running
                }
                _ => LoopState::Running,
            }
        })
    }

    fn finished(&mut self) {
        (self.callback)(&self.record);
    }
}