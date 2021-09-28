extern crate alloc;

use super::{ControlHardware, ControlLoop, LoopState};
use crate::{
    comms::{fdcan::FdcanMessage, messages::ExtendedFdcanFrame},
    current_sensing::PhaseCurrents,
    pi_controller::PIController,
};
use alloc::boxed::Box;

// Field-oriented control. Very basic Park/Clark forward and inverse. Currently no SVM is performed,
// and only a single i_q/i_d value is accepted S

const TWO_THIRDS: f32 = 0.6666666666666;
const SQRT_3: f32 = 1.73205080757;
const FRAC_SQRT_3_2: f32 = SQRT_3 / 2.;
// TODO(blakely): Hardcoded here
const DT: f32 = 1. / 40_000.;
const MIN_PWM_VALUE: f32 = 0.;
const MAX_PWM_VALUE: f32 = 2125.;
const PWM_INVERT: bool = true;

pub struct DQCurrents {
    pub q: f32,
    pub d: f32,
}

struct DQVoltages {
    q: f32,
    d: f32,
}

struct PhaseVoltages {
    a: f32,
    b: f32,
    c: f32,
}
struct PhaseDuty {
    a: f32,
    b: f32,
    c: f32,
}

pub struct CalibrateEZero<'a> {
    currents: DQCurrents,
    q_controller: PIController,
    d_controller: PIController,

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
        // TODO(blakely): Don't hard-code these; instead pull from either global config,
        // calibration, or FDCAN command.
        CalibrateEZero {
            currents,
            q_controller: PIController::new(1.421142407046769, 0.055681818, 24.),
            d_controller: PIController::new(1.421142407046769, 0.055681818, 24.),
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

fn forward_park_clark(phase_currents: PhaseCurrents, cos: f32, sin: f32) -> DQCurrents {
    let half_cos = cos / 2.;
    let half_sin = sin / 2.;

    let PhaseCurrents {
        phase_a: a,
        phase_b: b,
        phase_c: c,
        ..
    } = phase_currents;

    // Simplified DQ0 transform.
    let d = TWO_THIRDS
        * (cos * a + (FRAC_SQRT_3_2 * sin - half_cos) * b + (-FRAC_SQRT_3_2 * sin - half_cos) * c);
    let q = TWO_THIRDS
        * (-sin * a - (-FRAC_SQRT_3_2 * cos - half_sin) * b - (FRAC_SQRT_3_2 * cos - half_sin) * c);
    DQCurrents { d, q }
}

fn inverse_park_clark(dq_voltages: DQVoltages, cos: f32, sin: f32) -> PhaseVoltages {
    let half_cos = cos / 2.;
    let half_sin = sin / 2.;

    let DQVoltages { d, q } = dq_voltages;

    let a = cos * d - sin * q;
    let b = (FRAC_SQRT_3_2 * sin - half_cos) * d - (-FRAC_SQRT_3_2 * cos - half_sin) * q;
    let c = (-FRAC_SQRT_3_2 * sin - half_cos) * d - (FRAC_SQRT_3_2 * cos - half_sin) * q;
    PhaseVoltages { a, b, c }
}

fn space_vector_modulation(v_ref: f32, phase_voltages: PhaseVoltages) -> PhaseDuty {
    let PhaseVoltages {
        a: a_raw,
        b: b_raw,
        c: c_raw,
    } = phase_voltages;
    let v_min = a_raw.min(b_raw).min(c_raw);
    let v_max = a_raw.max(b_raw).max(c_raw);
    let v_offset = (v_min + v_max) / 2.;
    // const PWM_OFFSET: f32 = (0.0 + 0.94) / 2.;
    let pwm_offset: f32 = (0.0 + 0.94) / 2.;
    let a = (0.5 * (a_raw - v_offset) / v_ref + pwm_offset)
        .max(0.)
        .min(0.94);
    let b = (0.5 * (b_raw - v_offset) / v_ref + pwm_offset)
        .max(0.)
        .min(0.94);
    let c = (0.5 * (c_raw - v_offset) / v_ref + pwm_offset)
        .max(0.)
        .min(0.94);
    PhaseDuty { a, b, c }
}

impl<'a> ControlLoop for CalibrateEZero<'a> {
    fn commutate(&mut self, hardware: &mut ControlHardware) -> LoopState {
        let encoder = &hardware.encoder;
        let cordic = &mut hardware.cordic;
        // Kick off CORDIC conversion
        let pending_cos_sin = cordic.cos_sin(encoder.electrical_angle);

        let _asdf = encoder.electrical_angle.in_radians();
        // Sample ADCs in the meantime
        let phase_currents = hardware.current_sensor.sample();
        // Actually get the results of the Cos/Sin transform.
        let [cos, sin] = pending_cos_sin.get_result();
        // Calculate the park/clark currents
        let dq_currents = forward_park_clark(phase_currents, cos, sin);

        // Kick off new CORDIC conversion for future electrical theta
        // TODO(blakely): Why does Ben use 1.5x here?
        let new_electrical_theta =
            encoder.electrical_angle + 1.5f32 * DT * encoder.electrical_velocity;
        let pending_cos_sin = cordic.cos_sin(new_electrical_theta);
        // In the meantime, update the controllers for d and q axes
        let new_q_voltage = self.q_controller.update(dq_currents.q, self.currents.q);
        let new_d_voltage = self.d_controller.update(dq_currents.d, self.currents.d);
        // Get the result of the new theta.
        let [cos, sin] = pending_cos_sin.get_result();
        let new_voltages = inverse_park_clark(
            DQVoltages {
                q: new_q_voltage,
                d: new_d_voltage,
            },
            cos,
            sin,
        );

        // Get the current rail voltage.
        let v_bus = hardware.current_sensor.v_bus();
        let tim1 = &hardware.tim1;

        let pwms = match PWM_INVERT {
            false => PhaseDuty {
                a: new_voltages.a / v_bus * 0.5 + 0.5,
                b: new_voltages.b / v_bus * 0.5 + 0.5,
                c: new_voltages.c / v_bus * 0.5 + 0.5,
            },
            true => PhaseDuty {
                a: -new_voltages.a / v_bus * 0.5 + 0.5,
                b: -new_voltages.b / v_bus * 0.5 + 0.5,
                c: -new_voltages.c / v_bus * 0.5 + 0.5,
            },
        };

        // Set PWM values
        tim1.ccr1.write(|w| w.ccr1().bits((pwms.a * 2125.) as u16));
        tim1.ccr2.write(|w| w.ccr2().bits((pwms.b * 2125.) as u16));
        tim1.ccr3.write(|w| w.ccr3().bits((pwms.c * 2125.) as u16));

        let angle_raw = match encoder.angle_state().raw_angle {
            Some(x) => x as u32,
            None => 0,
        };

        self.record = EZeroMsg {
            angle: encoder.angle_state().angle.in_radians(),
            angle_raw,
            e_angle: encoder.electrical_angle.in_radians(),
            e_raw: angle_raw as f32 / 21.,
        };

        self.loop_count += 1;
        match self.loop_count {
            x if x >= self.total_counts => {
                self.currents.q = 0.;
                self.currents.q = 0.;
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
    }

    fn finished(&mut self) {
        (self.callback)(&self.record);
    }
}

pub struct CalibrateEZeroMsg {
    pub duration: f32,
    pub currents: DQCurrents,
}

impl ExtendedFdcanFrame for CalibrateEZeroMsg {
    fn pack(&self) -> crate::comms::fdcan::FdcanMessage {
        panic!("Pack not supported")
    }
    fn unpack(message: &crate::comms::fdcan::FdcanMessage) -> Self {
        let buffer = message.data;
        CalibrateEZeroMsg {
            duration: f32::from_bits(buffer[0]),
            currents: DQCurrents {
                q: f32::from_bits(buffer[1]),
                d: f32::from_bits(buffer[2]),
            },
        }
    }
}

pub struct EZeroMsg {
    e_angle: f32,
    e_raw: f32,
    angle: f32,
    angle_raw: u32,
}

impl<'a> ExtendedFdcanFrame for EZeroMsg {
    fn unpack(_: &FdcanMessage) -> Self {
        panic!("Unack not supported");
    }

    fn pack(&self) -> FdcanMessage {
        FdcanMessage::new(
            0x15,
            &[
                self.angle.to_bits(),
                self.angle_raw,
                self.e_angle.to_bits(),
                self.e_raw.to_bits(),
            ],
        )
    }
}
