use core::marker::PhantomData;
use core::ops;

use stm32g4::stm32g474 as device;

use crate::{block_until, block_while, util::stm32::blocking_sleep_us};

// TODO(blakely): Generalize this with HAL
pub struct CurrentSensor<T: CurrentSensorState> {
    phase_a: device::ADC1,
    phase_a_offset: f32,
    phase_b: device::ADC2,
    phase_b_offset: f32,
    phase_c: device::ADC3,
    phase_c_offset: f32,

    sense_gain: f32,
    sense_v_ref: f32,

    v_bus: device::ADC4,
    v_bus_gain: f32,

    v_refint: device::ADC5,
    from_v_refint: fn(u16, u16) -> f32,
    _marker: PhantomData<T>,
}

pub trait CurrentSensorState {}

pub struct Configuring {}
impl CurrentSensorState for Configuring {}
pub struct Calibrating {}
impl CurrentSensorState for Calibrating {}
pub struct Ready {}
impl CurrentSensorState for Ready {}

pub fn new(
    phase_a: device::ADC1,
    phase_b: device::ADC2,
    phase_c: device::ADC3,
    v_bus: device::ADC4,
    v_refint: device::ADC5,
) -> CurrentSensor<Configuring> {
    let from_v_refint = |v_refint_reading: u16, adc_reading: u16| {
        const V_REFINT_CAL: f32 = 3.0;
        // Safety: According to STM32G474 datassheet, internal voltage reference calibration value
        // is a u16 located at 0x1FFF_75AA. Calibration was performed at 3.0V
        let v_refint_cal_value = unsafe { *((0x1FFF_75AA) as *const u16) };
        // TODO(blakely): assumes 12-bit precision
        (V_REFINT_CAL as f32 * v_refint_cal_value as f32 * adc_reading as f32)
            / (v_refint_reading as f32 * 4096f32)
    };

    CurrentSensor {
        phase_a,
        phase_a_offset: 0.,
        phase_b,
        phase_b_offset: 0.,
        phase_c,
        phase_c_offset: 0.,
        v_bus,
        v_bus_gain: 1.0,
        v_refint,
        from_v_refint,

        // TODO(blakely): These should be configurable.
        sense_gain: 1. / (40. * 0.001),
        sense_v_ref: 3.3,

        _marker: PhantomData,
    }
}

impl CurrentSensor<Configuring> {
    // TODO(blakely): Make this configurable after HAL is ready.
    pub fn configure_phase_sensing(mut self) -> Self {
        let adc1 = &self.phase_a;
        let adc2 = &self.phase_b;
        let adc3 = &self.phase_c;
        // Begin in a sane state.
        adc1.cr.modify(|_, w| {
            w.adcal()
                .clear_bit()
                .aden()
                .clear_bit()
                .adstart()
                .clear_bit()
                .advregen()
                .clear_bit()
        });
        adc2.cr.modify(|_, w| {
            w.adcal()
                .clear_bit()
                .aden()
                .clear_bit()
                .adstart()
                .clear_bit()
                .advregen()
                .clear_bit()
        });
        adc3.cr.modify(|_, w| {
            w.adcal()
                .clear_bit()
                .aden()
                .clear_bit()
                .adstart()
                .clear_bit()
                .advregen()
                .clear_bit()
        });

        // Wake from deep power down, enable ADC voltage regulator, and set single-ended input mode.
        adc1.cr
            .modify(|_, w| w.deeppwd().disabled().advregen().enabled());
        adc2.cr
            .modify(|_, w| w.deeppwd().disabled().advregen().enabled());
        adc3.cr
            .modify(|_, w| w.deeppwd().disabled().advregen().enabled());
        // Allow voltage regulators to warm up. Datasheet says 20us max, so let's do 30us to be
        // safe.
        blocking_sleep_us(30);

        // Can probably combine these modifies, but kept separate in case the clear bit has to be
        // set first.
        adc1.cr.modify(|_, w| w.aden().clear_bit());
        adc1.cr.modify(|_, w| w.adcaldif().single_ended());
        adc1.cr.modify(|_, w| w.adcal().set_bit());
        adc2.cr.modify(|_, w| w.aden().clear_bit());
        adc2.cr.modify(|_, w| w.adcaldif().single_ended());
        adc2.cr.modify(|_, w| w.adcal().set_bit());
        adc3.cr.modify(|_, w| w.aden().clear_bit());
        adc3.cr.modify(|_, w| w.adcaldif().single_ended());
        adc3.cr.modify(|_, w| w.adcal().set_bit());

        // Wait for it to complete
        // Datasheet table 66 suggests t_cal is 116*1/f_adc. Since we're using a 4x clock
        // downscaling, that results in 1/(170e6/4) * 1e6 * 116 = 2.7294 us. Might as well block for
        // 10us just to be safe.
        blocking_sleep_us(10);
        block_until!(adc1.cr.read().adcal().bit_is_clear());
        block_until!(adc2.cr.read().adcal().bit_is_clear());
        block_until!(adc3.cr.read().adcal().bit_is_clear());

        // Check that we're ready, enable, and wait for ready state. Initial adrdy.set_bit is to
        // ensure it's cleared.
        adc1.isr.modify(|_, w| w.adrdy().set_bit());
        adc1.cr.modify(|_, w| w.aden().set_bit());
        adc2.isr.modify(|_, w| w.adrdy().set_bit());
        adc2.cr.modify(|_, w| w.aden().set_bit());
        adc3.isr.modify(|_, w| w.adrdy().set_bit());
        adc3.cr.modify(|_, w| w.aden().set_bit());

        // Wait for ready
        block_until!(adc1.isr.read().adrdy().bit_is_set());
        block_until!(adc2.isr.read().adrdy().bit_is_set());
        block_until!(adc3.isr.read().adrdy().bit_is_set());

        // Clear ready, for good measure.
        adc1.isr.modify(|_, w| w.adrdy().set_bit());
        adc2.isr.modify(|_, w| w.adrdy().set_bit());
        adc3.isr.modify(|_, w| w.adrdy().set_bit());

        // Configure channels

        // ADC[123] - Current sense amplifiers. Single channel inputs, and triggered by `tim_trgo2`.
        adc1.cr.modify(|_, w| w.adstart().clear_bit());
        adc2.cr.modify(|_, w| w.adstart().clear_bit());
        adc3.cr.modify(|_, w| w.adstart().clear_bit());
        // Note that L=0 implies 1 conversion.
        // Safety: SVD doesn't have valid range for this, so we're "arbitrarily setting bits". As
        // long as it's 0-16 for L and 0-18 for SQx, we should be good.
        adc1.sqr1
            .modify(|_, w| unsafe { w.l().bits(0).sq1().bits(2) });
        adc2.sqr1
            .modify(|_, w| unsafe { w.l().bits(0).sq1().bits(1) });
        adc3.sqr1
            .modify(|_, w| unsafe { w.l().bits(0).sq1().bits(1) });
        // Fastest sample time we can, since there should be little-to-no resistance coming in from
        // the DRV current sense amplifier.
        adc1.smpr1.modify(|_, w| w.smp2().cycles2_5());
        adc2.smpr1.modify(|_, w| w.smp1().cycles2_5());
        adc3.smpr1.modify(|_, w| w.smp1().cycles2_5());

        self.set_conversion_on_trgo2();

        self
    }

    pub fn set_conversion_on_trgo2(&mut self) {
        self.phase_a.cfgr.modify(|_, w| {
            w.res()
                .bits12()
                .exten()
                .rising_edge()
                .extsel()
                .tim1_trgo2()
                .align()
                .right()
                .cont()
                .single()
                .discen()
                .disabled()
                .ovrmod()
                .overwrite()
        });
        self.phase_b.cfgr.modify(|_, w| {
            w.res()
                .bits12()
                .exten()
                .rising_edge()
                .extsel()
                .tim1_trgo2()
                .align()
                .right()
                .cont()
                .single()
                .discen()
                .disabled()
                .ovrmod()
                .overwrite()
        });
        self.phase_c.cfgr.modify(|_, w| {
            w.res()
                .bits12()
                .exten()
                .rising_edge()
                .extsel()
                .tim1_trgo2()
                .align()
                .right()
                .cont()
                .single()
                .discen()
                .disabled()
                .ovrmod()
                .overwrite()
        });
    }

    pub fn set_single_conversion(&mut self) {
        self.phase_a.cfgr.modify(|_, w| {
            w.res()
                .bits12()
                .exten()
                .disabled()
                .align()
                .right()
                .cont()
                .single()
                .discen()
                .disabled()
                .ovrmod()
                .overwrite()
        });
        self.phase_b.cfgr.modify(|_, w| {
            w.res()
                .bits12()
                .exten()
                .disabled()
                .align()
                .right()
                .cont()
                .single()
                .discen()
                .disabled()
                .ovrmod()
                .overwrite()
        });
        self.phase_c.cfgr.modify(|_, w| {
            w.res()
                .bits12()
                .exten()
                .disabled()
                .align()
                .right()
                .cont()
                .single()
                .discen()
                .disabled()
                .ovrmod()
                .overwrite()
        });
    }

    pub fn configure_v_bus(mut self, v_bus_gain: f32) -> Self {
        let adc4 = &self.v_bus;
        // Begin in a sane state.
        adc4.cr.modify(|_, w| {
            w.adcal()
                .clear_bit()
                .aden()
                .clear_bit()
                .adstart()
                .clear_bit()
                .advregen()
                .clear_bit()
        });

        // Wake from deep power down, enable ADC voltage regulator, and set single-ended input mode.
        adc4.cr
            .modify(|_, w| w.deeppwd().disabled().advregen().enabled());

        // Allow voltage regulators to warm up. Datasheet says 20us max.
        blocking_sleep_us(20);

        // Begin calibration
        adc4.cr.modify(|_, w| w.aden().clear_bit());
        adc4.cr.modify(|_, w| w.adcaldif().single_ended());
        adc4.cr.modify(|_, w| w.adcal().set_bit());

        // Wait for it to complete
        block_until!(adc4.cr.read().adcal().bit_is_clear());

        // Check that we're ready, enable, and wait for ready state. Initial adrdy.set_bit is to
        // ensure it's cleared.
        adc4.isr.modify(|_, w| w.adrdy().set_bit());
        adc4.cr.modify(|_, w| w.aden().set_bit());

        // Wait for ready
        block_until!(adc4.isr.read().adrdy().bit_is_set());

        // Clear ready, for good measure.
        adc4.isr.modify(|_, w| w.adrdy().set_bit());

        // ADC4 ( V_BUS sampling) only uses a single channel: IN3
        // Safety: SVD doesn't have valid range for this, so we're "arbitrarily setting bits". As
        // long as it's 0-16 for L and 0-18 for SQx, we should be good.
        adc4.cr.modify(|_, w| w.adstart().clear_bit());
        adc4.sqr1
            .modify(|_, w| unsafe { w.l().bits(0).sq1().bits(3) });
        // There's quite a bit of input resistance on the Vbus line. Datasheet suggests 39kOhm is
        // the upper limit for 60MHz sampling. We're using 42.5 and doing a single channel, so we
        // should be somewhat clear sampling for longer.
        adc4.smpr1.modify(|_, w| w.smp3().cycles640_5());
        // Set 12-bit continuous conversion mode with right-data-alignment, and ensure that no
        // hardware trigger is used. Also set overrun mode to allow overwrites of the data register,
        // otherwise it'll pause after one.
        adc4.cfgr.modify(|_, w| {
            w.res()
                .bits12()
                .cont()
                .continuous()
                .align()
                .right()
                .exten()
                .disabled()
                .ovrmod()
                .overwrite()
        });

        self.v_bus_gain = v_bus_gain;

        self
    }

    pub fn configure_v_refint(self) -> Self {
        let adc5 = &self.v_refint;
        // Begin in a sane state.
        adc5.cr.modify(|_, w| {
            w.adcal()
                .clear_bit()
                .aden()
                .clear_bit()
                .adstart()
                .clear_bit()
                .advregen()
                .clear_bit()
        });

        // Wake from deep power down, enable ADC voltage regulator, and set single-ended input mode.
        adc5.cr
            .modify(|_, w| w.deeppwd().disabled().advregen().enabled());

        // Allow voltage regulators to warm up. Datasheet says 20us max.
        blocking_sleep_us(20);

        // Begin calibration
        adc5.cr.modify(|_, w| w.aden().clear_bit());
        adc5.cr.modify(|_, w| w.adcaldif().single_ended());
        adc5.cr.modify(|_, w| w.adcal().set_bit());

        // Wait for it to complete
        block_until!(adc5.cr.read().adcal().bit_is_clear());

        // Check that we're ready, enable, and wait for ready state. Initial adrdy.set_bit is to
        // ensure it's cleared.
        adc5.isr.modify(|_, w| w.adrdy().set_bit());
        adc5.cr.modify(|_, w| w.aden().set_bit());

        // Wait for ready
        block_until!(adc5.isr.read().adrdy().bit_is_set());
        // Clear ready, for good measure.
        adc5.isr.modify(|_, w| w.adrdy().set_bit());

        // ADC5 (V_REFINT) Similar to ADC4 above, but using IN18
        adc5.cr.modify(|_, w| w.adstart().clear_bit());
        adc5.sqr1
            .modify(|_, w| unsafe { w.l().bits(0).sq1().bits(18) });
        adc5.smpr2.modify(|_, w| w.smp18().cycles640_5());
        adc5.cfgr.modify(|_, w| {
            w.res()
                .bits12()
                .cont()
                .set_bit()
                .align()
                .right()
                .exten()
                .disabled()
                .ovrmod()
                .overwrite()
        });

        self
    }

    pub fn ready(self) -> CurrentSensor<Ready> {
        // Clear pending signals
        self.phase_a
            .isr
            .modify(|_, w| w.eosmp().set_bit().eoc().set_bit().ovr().set_bit());
        // Start up ADCs.
        self.phase_a.cr.modify(|_, w| w.adstart().set_bit());
        self.phase_b.cr.modify(|_, w| w.adstart().set_bit());
        self.phase_c.cr.modify(|_, w| w.adstart().set_bit());
        // Enable interrupt on ADC1 EOS. Only needed for ADC1, since 2 and 3 are sync'd to the same
        // tim_trgo2 _and_ have the same sampling period.
        self.phase_a.ier.modify(|_, w| w.eosie().enabled());

        // Start continuous sampling on v_bus and v_refint
        self.v_bus.cr.modify(|_, w| w.adstart().set_bit());
        self.v_refint.cr.modify(|_, w| w.adstart().set_bit());
        // Make sure we've got a reading on v_bus and v_refint
        block_until!(self.v_bus.isr.read().eoc().is_complete());
        block_until!(self.v_refint.isr.read().eoc().is_complete());

        CurrentSensor {
            phase_a: self.phase_a,
            phase_a_offset: 0.,
            phase_b: self.phase_b,
            phase_b_offset: 0.,
            phase_c: self.phase_c,
            phase_c_offset: 0.,
            sense_gain: self.sense_gain,
            sense_v_ref: self.sense_v_ref,

            v_bus: self.v_bus,
            v_bus_gain: self.v_bus_gain,
            v_refint: self.v_refint,

            from_v_refint: self.from_v_refint,

            _marker: PhantomData,
        }
    }
}

impl CurrentSensor<Ready> {
    pub fn acknowledge_eos(&mut self) {
        // Clear the EOS flag from ADC1, what we're using to trigger the control loop interrupt.
        // Note: `clear()` is a bad name, since it doesn't clear the _bit_, but clears the _flag_ by
        // writing a 1.
        self.phase_a.isr.modify(|_, w| w.eos().clear());
    }
}

// Sample ADCs and offset current values by calibrated offsets.
fn sample<T: CurrentSensorState>(sensor: &CurrentSensor<T>) -> CurrentMeasurement {
    let mut measurement = sample_raw(sensor);
    measurement.phase_a -= sensor.phase_a_offset;
    measurement.phase_b -= sensor.phase_b_offset;
    measurement.phase_c -= sensor.phase_c_offset;
    measurement
}

// Sample ADCs values directly, not applying any offsets.
fn sample_raw<T: CurrentSensorState>(sensor: &CurrentSensor<T>) -> CurrentMeasurement {
    let v_refint = sensor.v_refint.dr.read().bits() as u16;
    let sense_v_ref_over_2: f32 = sensor.sense_v_ref / 2.0;

    let calc_phase_current = |adc_reading: u16| -> f32 {
        let adc_voltage = (sensor.from_v_refint)(v_refint, adc_reading as u16);
        (sense_v_ref_over_2 - adc_voltage) * sensor.sense_gain
    };
    let phase_a = sensor.phase_a.dr.read().rdata().bits();
    let phase_b = sensor.phase_b.dr.read().rdata().bits();
    let phase_c = sensor.phase_c.dr.read().rdata().bits();
    let phase_a = calc_phase_current(phase_a);
    let phase_b = calc_phase_current(phase_b);
    let phase_c = calc_phase_current(phase_c);
    let v_bus =
        (sensor.from_v_refint)(v_refint, sensor.v_bus.dr.read().bits() as u16) * sensor.v_bus_gain;

    CurrentMeasurement {
        phase_a,
        phase_b,
        phase_c,
        v_bus,
    }
}

pub struct CurrentMeasurement {
    pub phase_a: f32,
    pub phase_b: f32,
    pub phase_c: f32,
    pub v_bus: f32,
}

impl CurrentMeasurement {
    pub fn new() -> CurrentMeasurement {
        CurrentMeasurement {
            phase_a: 0.,
            phase_b: 0.,
            phase_c: 0.,
            v_bus: 0.,
        }
    }
}

impl CurrentSensor<Ready> {
    // Sample ADC values and correct for offset.
    pub fn sample(&self) -> CurrentMeasurement {
        sample(self)
    }

    // Sample raw ADC values (no offset correction).
    pub fn sample_raw(&self) -> CurrentMeasurement {
        sample_raw(self)
    }

    pub fn set_calibration(&mut self, phase_a: f32, phase_b: f32, phase_c: f32) {
        self.phase_a_offset = phase_a;
        self.phase_b_offset = phase_b;
        self.phase_c_offset = phase_c;
    }
}

impl ops::Add for CurrentMeasurement {
    type Output = CurrentMeasurement;

    fn add(self, rhs: CurrentMeasurement) -> Self::Output {
        CurrentMeasurement {
            phase_a: self.phase_a + rhs.phase_a,
            phase_b: self.phase_b + rhs.phase_b,
            phase_c: self.phase_c + rhs.phase_c,
            v_bus: self.v_bus + rhs.v_bus,
        }
    }
}

impl ops::AddAssign for CurrentMeasurement {
    fn add_assign(&mut self, rhs: Self) {
        self.phase_a += rhs.phase_a;
        self.phase_b += rhs.phase_b;
        self.phase_c += rhs.phase_c;
        self.v_bus += rhs.v_bus;
    }
}

impl ops::DivAssign<u32> for CurrentMeasurement {
    fn div_assign(&mut self, rhs: u32) {
        self.phase_a /= rhs as f32;
        self.phase_b /= rhs as f32;
        self.phase_c /= rhs as f32;
        self.v_bus /= rhs as f32;
    }
}
