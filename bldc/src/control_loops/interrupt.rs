use stm32g4::stm32g474::{self as device, interrupt};
use third_party::m4vga_rs::util::armv7m::clear_pending_irq;
use third_party::m4vga_rs::util::sync::acquire_hw;

use crate::led::{self, Led};

use super::controller::{
    Commutate, ControlLoop, InterruptData, INTERRUPT_SHARED, LOOP_STATE, SENSOR_STATE,
};
use super::{ControlHardware, LoopState, SensorState};

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
    let shared = &mut *acquire_hw(&INTERRUPT_SHARED);
    let InterruptData {
        ref mut control_loop,
        ref mut hw,
    } = shared;

    // Identify current state of the BLDC.
    let ControlHardware {
        ref mut current_sensor,
        ref mut encoder,
        ..
    } = hw;

    // First off: acknowledge the end of sampling signal in the ADC. Required otherwise the ADC
    // will immediately trigger another interrupt, regardless of whether the IRQ was cleared in
    // the NVIC above.
    current_sensor.acknowledge_eos();

    // Next, grab the encoder angle and update velocity and acceleration.

    // TODO(blakely): pull the frequency from the commutation state.
    let encoder_state = encoder.update(1. / 40000.);

    // Sample ADCs in the meantime
    let phase_currents = current_sensor.sample();

    // Get the current rail voltage.
    let v_bus = current_sensor.v_bus();

    // Update the state
    *SENSOR_STATE.lock_write() = Some(SensorState::new(&encoder_state, &phase_currents, v_bus));

    // If there's a control callback, call it. Otherwise just idle.
    let control_loop: &mut ControlLoop = match control_loop {
        None => return,
        Some(ref mut x) => x,
    };

    // Unwrap should be fine here. We're in the interrupt handler and the highest priority, so
    // nothing should be able to preempt us between when we set it above and now.
    let sensor_state = &SENSOR_STATE.read().unwrap();

    let new_loop_state = match control_loop.commutate(LOOP_STATE.read(), sensor_state, hw) {
        LoopState::Idle => {
            let pwm = &mut hw.pwm;
            // Make sure we pull all phases low in case the control loops didn't. Better safe than
            // sorry...
            pwm.zero_phases();

            // Reset the current sampling to be between PWM pulses.
            pwm.reset_current_sample();
            pwm.reset_deadtime();
            control_loop.finished();
            shared.control_loop = None;
            LoopState::Idle
        }
        x => x,
    };
    *LOOP_STATE.lock_write() = new_loop_state;
}
