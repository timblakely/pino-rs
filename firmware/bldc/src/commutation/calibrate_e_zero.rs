extern crate alloc;

use super::{ControlHardware, ControlLoop, LoopState};
use crate::{
    comms::{
        fdcan::{FdcanMessage, IncomingFdcanFrame, OutgoingFdcanFrame},
        messages::Message,
    },
    foc::{DQCurrents, FieldOrientedControlImpl},
    led::Led,
    pi_controller::PIController,
};
use alloc::boxed::Box;

// Field-oriented control. Very basic Park/Clark forward and inverse. Currently no SVM is performed,
// and only a single i_q/i_d value is accepted S

// TODO(blakely): Hardcoded here
const DT: f32 = 1. / 40_000.;
const _MIN_PWM_VALUE: f32 = 0.;
const _MAX_PWM_VALUE: f32 = 2125.;

pub struct CalibrateEZero<'a> {
    foc: FieldOrientedControlImpl,

    total_counts: u32,
    loop_count: u32,

    record: EZeroMsg,

    callback: Box<dyn for<'r> FnMut(&'r EZeroMsg) + 'a + Send>,
}

impl<'a> CalibrateEZero<'a> {
    pub fn new(
        duration: f32,
        currents: DQCurrents,
        callback: impl for<'r> FnMut(&'r EZeroMsg) + 'a + Send,
    ) -> CalibrateEZero<'a> {
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
            callback: Box::new(callback),

            record: EZeroMsg {
                angle: 0.,
                angle_raw: 0,
                e_angle: 0.,
                e_raw: 0.,
            },
        }
    }
}

impl<'a> ControlLoop for CalibrateEZero<'a> {
    fn commutate(&mut self, hardware: &mut ControlHardware) -> LoopState {
        Led::<crate::led::Red>::on_while(|| {
            // Get the current rail voltage.
            let v_bus = hardware.current_sensor.v_bus();

            // Calculate the required PWM values via field oriented control.
            let phase_voltages = self.foc.update(
                &hardware.current_sensor,
                &hardware.encoder,
                &mut hardware.cordic,
                DT,
            );
            hardware.pwm.set_voltages(v_bus, phase_voltages);

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

pub struct CalibrateEZeroCmd {
    pub duration: f32,
    pub currents: DQCurrents,
}

impl IncomingFdcanFrame for CalibrateEZeroCmd {
    fn unpack(message: FdcanMessage) -> Self {
        let buffer = message.data;
        CalibrateEZeroCmd {
            duration: f32::from_bits(buffer[0]),
            currents: DQCurrents {
                q: f32::from_bits(buffer[1]),
                d: f32::from_bits(buffer[2]),
            },
        }
    }
}

pub struct EZeroMsg {
    pub e_angle: f32,
    pub e_raw: f32,
    pub angle: f32,
    pub angle_raw: u32,
}

impl<'a> OutgoingFdcanFrame for EZeroMsg {
    fn pack(&self) -> FdcanMessage {
        FdcanMessage::new(
            Message::EZero,
            &[
                self.angle.to_bits(),
                self.angle_raw,
                self.e_angle.to_bits(),
                self.e_raw.to_bits(),
            ],
        )
    }
}
