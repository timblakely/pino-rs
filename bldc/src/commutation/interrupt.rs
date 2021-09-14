use stm32g4::stm32g474::{self as device, interrupt};
use third_party::m4vga_rs::util::armv7m::clear_pending_irq;

use super::{Commutator, LoopState, CONTROL_LOOP};

// Interrupt handler triggered by TIM1[CH4]'s tim_trgo2. Under normal circumstances this function
// will be called continuously, regardless of the control loop in place. Note that the control loop
// itself can modify the timings here since it has access to the underlying timer. Thus it's
// important that any modifications that are done by the control loop are un-done on completion.
#[interrupt]
fn ADC1_2() {
    // Clear the IRQ so it doesn't immediately fire again.
    clear_pending_irq(device::Interrupt::ADC1_2);
    // Main control loop.
    unsafe {
        *(0x4800_0418 as *mut u32) = 1 << 9;
        cortex_m::asm::nop();
        cortex_m::asm::nop();
        cortex_m::asm::nop();
        cortex_m::asm::nop();
        cortex_m::asm::nop();
        cortex_m::asm::nop();
        cortex_m::asm::nop();
        cortex_m::asm::nop();
        cortex_m::asm::nop();
        cortex_m::asm::nop();
        *(0x4800_0418 as *mut u32) = 1 << (9 + 16);
    }

    // If there's a control callback, call it. Otherwise just idle.
    let mut loop_vars = CONTROL_LOOP.lock();
    let mut loop_vars = loop_vars.as_mut().expect("Loop variables not set");

    // Required otherwise the ADC will immediately trigger another interrupt, regardless of whether
    // the IRQ was cleared in the NVIC above.
    loop_vars.hw.current_sensor.acknowledge_eos();

    let commutator = match loop_vars.control_loop {
        Some(ref mut x) => x,
        _ => return,
    };

    match commutator.commutate(&mut loop_vars.hw) {
        LoopState::Finished => {
            let tim1 = &loop_vars.hw.tim1;
            // Make sure we pull all phases low in case the control loops didn't. Better safe than
            // sorry...
            tim1.ccr1.write(|w| w.ccr1().bits(0));
            tim1.ccr2.write(|w| w.ccr2().bits(0));
            tim1.ccr3.write(|w| w.ccr3().bits(0));
            // Reset the current sampling to be between PWM pulses.
            tim1.ccr4.write(|w| w.ccr4().bits(2124));
            // Min deadtime required is 400ns, so make sure we're at least <= 98.3% duty cycle.
            // Safety: Upstream doesn't have an appropriate range. It's a 16 bit value, so 2083
            // should be safe.
            tim1.ccr5.write(|w| unsafe { w.ccr5().bits(2083) });
            commutator.finished();
            loop_vars.control_loop = None;
        }
        _ => return,
    }
}

impl Commutator {
    pub fn enable_loop() {
        {
            let mut control_vars = CONTROL_LOOP.lock();
            let hw = &control_vars
                .as_mut()
                .expect("Control loop vars not ready")
                .hw;
            // Kick off the timer.
            hw.tim1.cr1.modify(|_, w| w.cen().set_bit());
            // Now that the timer has started, enable the main output to allow current on the pins. If
            // we do this before we enable the timer, we have the potential to get into a state where the
            // PWM pins are in an active state but the timer isn't running, potentially drawing tons of
            // current through any high phases to any low phases.
            hw.tim1.bdtr.modify(|_, w| w.moe().set_bit());
        };
    }

    pub fn disable_loop() {
        {
            let mut control_vars = CONTROL_LOOP.lock();
            let hw = &control_vars
                .as_mut()
                .expect("Control loop vars not ready")
                .hw;
            hw.tim1.bdtr.modify(|_, w| w.moe().clear_bit());
            hw.tim1.cr1.modify(|_, w| w.cen().clear_bit());
        };
    }
}
