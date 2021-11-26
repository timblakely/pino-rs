use super::{Commutate, ControlHardware, LoopState, SensorState};
use crate::{
    comms::fdcan::{FdcanMessage, IncomingFdcanFrame, OutgoingFdcanFrame},
    current_sensing::PhaseCurrents,
    pwm::PwmDuty,
};

// Drive a zero-centered square wave through the phases, which should result in a triangle wave of
// current through the inductor. Measuring current over time should give us the inductance via
// V=L(di/dt). This appoximation holds as long as the square wave duration is ⋘ inductor time
// constant τ.

// Note: Since there is no current control in this loop, the PWM duty cycle is hard-coded to be
// 0.15, which at 24V and a winding resistance of 0.2R yields a peak of 7.5A at a square wave
// frequency of 4kHz. The windings I'm using are rated for 8A continuous and would very likely be
// able to handle much more for brief periods, but I ***really*** don't want to fry anything right
// now :)
const MAX_PWM_DUTY_CYCLE: f32 = 0.15;
const MIN_SQUARE_WAVE_FREQ: u32 = 5000;

// Measure the inductance of the windings
pub struct MeasureInductanceCmd {
    pub duration: f32,
    pub frequency: u32,
    pub pwm_duty: f32,
    pub sample_pwm_percent: f32,
}
// Return value for inductances
pub struct Inductances<'a> {
    pub inductances: &'a [f32; 3],
}

enum Direction {
    Up,
    Down,
}

pub struct MeasureInductance {
    total_counts: u32,
    loop_count: u32,
    direction: Direction,

    sample: PhaseCurrents,
    v_bus: f32,
    switch_count: u32,
    loops_per_switch: f32,
    remainder: f32,
    last_sample: Option<PhaseCurrents>,
    pwm_duty: f32,
    sample_pwm_ccr: u16,

    callback: fn([f32; 3]),

    switches: u32,
}

impl MeasureInductance {
    pub fn new(
        duration: f32,
        square_wave_freq: u32,
        pwm_duty: f32,
        sample_pwm_percent: f32,
        callback: fn([f32; 3]),
    ) -> MeasureInductance {
        let square_wave_freq = square_wave_freq.min(20_000).max(MIN_SQUARE_WAVE_FREQ);
        let pwm_duty = pwm_duty.max(0.).min(MAX_PWM_DUTY_CYCLE);
        if pwm_duty > MAX_PWM_DUTY_CYCLE {
            // TODO(blakely): This isn't a panic; this should be checked during `listen`.
            panic!("Max PWM duty cycle too high for inductance calibration")
        }
        let pwm_ccr = (pwm_duty * 2125f32) as u16;
        MeasureInductance {
            total_counts: (40_000 as f32 * duration) as u32,
            loop_count: 0,
            direction: Direction::Up,

            sample: PhaseCurrents::new(),
            v_bus: 0.,
            loops_per_switch: (40_000 as f32) / square_wave_freq as f32,
            switch_count: 0,
            remainder: 0.,
            last_sample: None,
            pwm_duty,
            sample_pwm_ccr: (((2125 - pwm_ccr) as f32 * sample_pwm_percent) as u16 + 1).max(2124),

            callback,

            switches: 0,
        }
    }
}

impl Commutate for MeasureInductance {
    fn commutate(
        &mut self,
        _loop_state: LoopState,
        _sensor_state: &SensorState,
        hardware: &mut ControlHardware,
    ) -> LoopState {
        let current_sensor = &mut hardware.current_sensor;
        current_sensor.sampling_period_long();

        self.v_bus += current_sensor.v_bus();
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
        let pwm = &mut hardware.pwm;

        pwm.set_sample_ccr(self.sample_pwm_ccr);

        if count_and_remainder >= self.loops_per_switch {
            self.switch_count = 0;
            self.switches += 1;
            self.remainder = count_and_remainder - self.loops_per_switch;
            self.direction = match self.direction {
                Direction::Up => {
                    pwm.set_pwm_duty_cycles(PwmDuty {
                        a: self.pwm_duty,
                        b: 0.,
                        c: 0.,
                    });
                    self.switches += 1;
                    Direction::Down
                }
                Direction::Down => {
                    pwm.set_pwm_duty_cycles(PwmDuty {
                        a: 0.,
                        b: self.pwm_duty,
                        c: self.pwm_duty,
                    });
                    self.switches += 1;
                    Direction::Up
                }
            };
        }

        self.loop_count += 1;
        match self.loop_count {
            x if x >= self.total_counts => {
                pwm.zero_phases();
                pwm.reset_current_sample();
                LoopState::Idle
            }
            _ => LoopState::Running,
        }
    }

    fn finished(&mut self) {
        let loop_count = self.loop_count as f32;
        let v_bus = self.v_bus / loop_count;
        let v_ref = v_bus * self.pwm_duty;
        let dt = loop_count / 40_000f32;

        let inductances = [
            v_ref / (self.sample.phase_a / dt),
            v_ref / (self.sample.phase_b / dt),
            v_ref / (self.sample.phase_c / dt),
        ];
        (self.callback)(inductances);
    }
}

impl IncomingFdcanFrame for MeasureInductanceCmd {
    fn unpack(message: FdcanMessage) -> Self {
        let buffer = message.data;
        MeasureInductanceCmd {
            duration: f32::from_bits(buffer[0]),
            frequency: buffer[1],
            pwm_duty: f32::from_bits(buffer[2]),
            sample_pwm_percent: f32::from_bits(buffer[3]),
        }
    }
}

impl<'a> OutgoingFdcanFrame for Inductances<'a> {
    fn pack(&self) -> FdcanMessage {
        FdcanMessage::new(
            0x14,
            &[
                self.inductances[0].to_bits(),
                self.inductances[1].to_bits(),
                self.inductances[2].to_bits(),
            ],
        )
    }
}
