#![no_std]
#![no_main]
#![feature(unboxed_closures, fn_traits)]

use bldc::{comms::messages::Messages, commutation::ControlParameters, driver};

#[cfg(feature = "panic-halt")]
use panic_halt as _;
#[cfg(feature = "panic-itm")]
use panic_itm as _; // you can put a breakpoint on `rust_begin_unwind` to catch panics

// TODO(blakely): Implement emergency stop.
fn emergency_stop() {}

// TODO(blakely): Comment on all the stuff that happens before we actually get
// here...
#[cortex_m_rt::entry]
fn main() -> ! {
    let initial_state = ControlParameters {
        pwm_duty: 0f32,
        q: 0f32,
        d: 0f32,
    };

    let controller = driver::take_hardware().configure_peripherals();

    let mut _sign = -1f32;
    let mut _square_wave_state: u32 = 0;
    controller.run(
        initial_state,
        |message, control_params| {
            match Messages::unpack_fdcan(message) {
                Some(Messages::ForcePwm(msg)) => control_params.pwm_duty = msg.pwm_duty,
                Some(Messages::EStop(_)) => emergency_stop(),
                Some(Messages::SetCurrents(msg)) => {
                    control_params.q = msg.q;
                    control_params.d = msg.d;
                }
                _ => {}
            };
        },
        // Control loop
        |hardware, control_params| {
            // This is called at 40kHz, and where any commutation happens.
            let new_arr: u16 = ((control_params.pwm_duty * 2125_f32) as u16).min(80).max(0);
            if new_arr <= 80 {
                hardware.tim1.ccr1.write(|w| w.ccr1().bits(new_arr));
            }

            // const CALIB_PWM_DUTY: f32 = 2. / 24.;
            // const CCR_2V: u16 = (CALIB_PWM_DUTY * 2125.) as u16;
            // match square_wave_state {
            //     0 => {
            //         // Switching states
            //         if sign < 0. {
            //             // Set A high and B and C low
            //             hardware.tim1.ccr1.write(|w| w.ccr1().bits(CCR_2V));
            //             hardware.tim1.ccr2.write(|w| w.ccr2().bits(0));
            //             hardware.tim1.ccr3.write(|w| w.ccr3().bits(0));
            //         } else {
            //             // Set A high and B and C low
            //             hardware.tim1.ccr1.write(|w| w.ccr1().bits(0));
            //             hardware.tim1.ccr2.write(|w| w.ccr2().bits(CCR_2V));
            //             hardware.tim1.ccr3.write(|w| w.ccr3().bits(CCR_2V));
            //         }
            //         sign = -sign;
            //     }
            //     _ => {
            //         square_wave_state += 1;
            //         if square_wave_state >= 2 {
            //             square_wave_state = 0;
            //         }
            //     }
            // };
        },
    );
    loop {}
}
