use stm32g4::stm32g474::{self as device, interrupt};
use third_party::m4vga_rs::util::armv7m::clear_pending_irq;

use crate::led::{self, Led};

use super::{
    CommutationLoop, ControlHardware, SensorState, COMMUTATING, COMMUTATION_STATE, SENSOR_STATE,
};

// Interrupt handler triggered by TIM1[CH4]'s tim_trgo2. Under normal circumstances this function
// will be called continuously, regardless of the control loop in place. Note that the control loop
// itself can modify the timings here since it has access to the underlying timer. Thus it's
// important that any modifications that are done by the control loop are un-done on completion.
#[interrupt]
fn ADC1_2() {
    // Main commutation loop.
    Led::<led::Blue>::on_while(|| {
        // Clear the IRQ so it doesn't immediately fire again.
        clear_pending_irq(device::Interrupt::ADC1_2);

        commutate();
    });
}

fn commutate() {
    let mut loop_vars = COMMUTATION_STATE.lock();
    let mut loop_vars = loop_vars.as_mut().expect("Loop variables not set");

    // Identify current state of the BLDC.
    {
        let ControlHardware {
            ref mut current_sensor,
            ref mut encoder,
            ..
        } = loop_vars.hw;

        // First off: acknowledge the end of sampling signal in the ADC. Required otherwise the ADC
        // will immediately trigger another interrupt, regardless of whether the IRQ was cleared in
        // the NVIC above.
        current_sensor.acknowledge_eos();

        // Next, grab the encoder angle and update velocity and acceleration.

        // TODO(blakely): pull the frequency from the commutation state.
        encoder.update(1. / 40000.);

        // Sample ADCs in the meantime
        let phase_currents = current_sensor.sample();

        // Get the current rail voltage.
        let v_bus = current_sensor.v_bus();

        // Update the state
        *SENSOR_STATE.lock_write() = Some(SensorState::new(
            encoder.angle_state(),
            encoder.state(),
            &phase_currents,
            encoder.observer_state(),
            v_bus,
        ));
    }

    // If there's a control callback, call it. Otherwise just idle.
    let commutator = match loop_vars.control_loop {
        Some(ref mut x) => x,
        _ => return,
    };

    COMMUTATING.store(true, core::sync::atomic::Ordering::Relaxed);

    // Unwrap should be fine here. We're in the interrupt handler and the highest priority, so
    // nothing should be able to preempt us between when we set it above and now.
    let sensor_state = &SENSOR_STATE.read().unwrap();

    match commutator.commutate(sensor_state, &mut loop_vars.hw) {
        CommutationLoop::Finished => {
            COMMUTATING.store(false, core::sync::atomic::Ordering::Relaxed);
            let pwm = &mut loop_vars.hw.pwm;
            // Make sure we pull all phases low in case the control loops didn't. Better safe than
            // sorry...
            pwm.zero_phases();

            // Reset the current sampling to be between PWM pulses.
            pwm.reset_current_sample();
            pwm.reset_deadtime();
            commutator.finished();
            loop_vars.control_loop = None;
        }
        _ => return,
    }
}
