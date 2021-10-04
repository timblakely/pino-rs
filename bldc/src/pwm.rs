use stm32g4::stm32g474 as device;

use crate::{block_until, block_while, timer::TimerConfig};

pub struct PwmDuty {
    pub a: f32,
    pub b: f32,
    pub c: f32,
}

pub struct PhaseVoltages {
    pub a: f32,
    pub b: f32,
    pub c: f32,
}

pub struct PwmOutput {
    timer: device::TIM1,
    invert: bool,
}

impl PwmOutput {
    pub fn new(timer: device::TIM1, invert_pwm: bool) -> PwmOutput {
        PwmOutput {
            timer,
            invert: invert_pwm,
        }
    }

    pub fn configure(&mut self, config: TimerConfig) {
        // Configure TIM1 for control loop (actual timer frequency is double, since up + down = 1
        // full cycle).
        let tim1 = &self.timer;
        // Stop the timer if it's running for some reason.
        tim1.cr1.modify(|_, w| w.cen().clear_bit());
        block_until!(tim1.cr1.read().cen().bit_is_clear());
        // Center-aligned mode 2: Up/Down and interrupts on up only.
        tim1.cr1
            .modify(|_, w| w.dir().up().cms().center_aligned2().ckd().div1());
        // Enable output state low on idle. Also set the master mode so that trgo2 is written based
        // on `tim_oc4refc`
        // Safety: mms2 doesn't have a valid range or enum set. Bits 0b0111 are tim_oc4refc.
        tim1.cr2.modify(|_, w| {
            unsafe {
                w.ccpc()
                    .clear_bit()
                    .ois1()
                    .clear_bit()
                    .ois2()
                    .clear_bit()
                    .ois3()
                    .clear_bit()
                    .ois4()
                    .clear_bit()
                    // Configure tim_oc4refc to be on ch4. Note that this must be on mms2 for trgo2!
                    .mms2()
                    .bits(0b0111)
            }
        });
        // Configure output channels to PWM mode 1. Note: OCxM registers are split between the first
        // three bits and the fourth bit. For PWM mode 1 the fourth bit should be zero which is the
        // reset value, but it's good practice to manually set it anyway.
        tim1.ccmr1_output().modify(|_, w| {
            w.cc1s()
                .output()
                .oc1m()
                .pwm_mode1()
                .oc1m_3()
                .clear_bit()
                .cc2s()
                .output()
                .oc2m()
                .pwm_mode1()
                .oc2m_3()
                .clear_bit()
        });
        tim1.ccmr2_output().modify(|_, w| {
            w.cc3s()
                .output()
                .oc3m()
                .pwm_mode1()
                .oc3m_3()
                .clear_bit()
                .cc4s()
                .output()
                .oc4m()
                .pwm_mode1()
                .oc4m_3()
                .clear_bit()
        });
        // Enable channels 1-5. 1-3 are the output pins, channel 4 is used to trigger the current
        // sampling, and 5 is used as the forced deadtime insertion. Set the output polarity to HIGH
        // (rising edge).
        tim1.ccer.modify(|_, w| {
            w.cc1e()
                .set_bit()
                .cc1p()
                .clear_bit()
                .cc2e()
                .set_bit()
                .cc2p()
                .clear_bit()
                .cc3e()
                .set_bit()
                .cc3p()
                .clear_bit()
                .cc4e()
                .set_bit()
                .cc4p()
                .clear_bit()
                .cc5e()
                .set_bit()
                .cc5p()
                .clear_bit()
        });
        // 80kHz@170MHz = Prescalar to 0, ARR to 2125
        // Note: the prescalar is 0-indexed; psc=0 implies prescalar = 1.
        tim1.psc.write(|w| w.psc().bits(config.prescalar - 1));
        tim1.arr.write(|w| w.arr().bits(config.arr));

        // Set repetition counter to 1, since we only want update TIM1 events on only after the full
        // up/down count cycle.
        // Safety: Upstream: needs range to be explicitly set for safety. 16-bit value.
        tim1.rcr.write(|w| unsafe { w.rep().bits(1) });

        // Set ccr values to 0 for all three channels.
        tim1.ccr1.write(|w| w.ccr1().bits(0));
        tim1.ccr2.write(|w| w.ccr2().bits(0));
        tim1.ccr3.write(|w| w.ccr3().bits(0));

        // Set channel 4 to trigger _just_ before the midway point.
        tim1.ccr4.write(|w| w.ccr4().bits(2124));
        // Set ch5 to PWM mode and enable it.
        // Safety: Upstream: needs enum values. PWM mode 1 is 0110.
        tim1.ccmr3_output
            .modify(|_, w| unsafe { w.oc5m().bits(110).oc5m_bit3().bits(0) });

        // Configure channels 1-3 to be logical AND'd with channel 5, and set its capture compare
        // value.
        // Safety: Upstream: needs range to be explicitly set for safety.
        // TODO(blakely): Set this CCR to a logical safe PWM duty (min deadtime 400ns = 98.4% duty
        // cycle at 40kHz)
        tim1.ccr5.modify(|_, w| unsafe {
            w.gc5c1()
                .set_bit()
                .gc5c2()
                .set_bit()
                .gc5c3()
                .set_bit()
                .ccr5()
                .bits(2083)
        });
    }

    pub fn set_pwm_duty_cycles(&mut self, pwms: PwmDuty) {
        // Set PWM values
        self.timer
            .ccr1
            .write(|w| w.ccr1().bits((pwms.a * 2125.) as u16));
        self.timer
            .ccr2
            .write(|w| w.ccr2().bits((pwms.b * 2125.) as u16));
        self.timer
            .ccr3
            .write(|w| w.ccr3().bits((pwms.c * 2125.) as u16));
    }

    pub fn set_voltages(&mut self, v_bus: f32, voltages: PhaseVoltages) {
        let pwms = match self.invert {
            false => PwmDuty {
                a: voltages.a / v_bus * 0.5 + 0.5,
                b: voltages.b / v_bus * 0.5 + 0.5,
                c: voltages.c / v_bus * 0.5 + 0.5,
            },
            true => PwmDuty {
                a: -voltages.a / v_bus * 0.5 + 0.5,
                b: -voltages.b / v_bus * 0.5 + 0.5,
                c: -voltages.c / v_bus * 0.5 + 0.5,
            },
        };
        self.set_pwm_duty_cycles(pwms);
    }
}
