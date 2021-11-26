#![cfg_attr(not(test), no_std)]
#![no_main]

use bldc::comms::handlers::disable_control_loop::DisableControlLoop;
use bldc::comms::handlers::pos_vel_control::EnterPosVelControl;
use bldc::comms::handlers::set_pos_vel::SetPosVel;
use bldc::comms::handlers::torque_control::EnterTorqueControl;
use bldc::driver;

#[cfg(feature = "panic-halt")]
use panic_halt as _;
#[cfg(feature = "panic-itm")]
use panic_itm as _; // you can put a breakpoint on `rust_begin_unwind` to catch panics

// TODO(blakely): Comment on all the stuff that happens before we actually get here...
#[cortex_m_rt::entry]
fn main() -> ! {
    // Acquire the driver.
    let mut driver = driver::take_hardware().configure_peripherals().calibrate();
    // driver.on_message(|message: FdcanMessage| match Message::parse(message) {
    //     Message::CalibrateEZero(_) => {
    //         fdcan::send_message(&EZeroMsg {
    //             angle: 123.,
    //             angle_raw: 456,
    //             e_angle: 789.,
    //             e_raw: 1337.,
    //         });
    //     }
    //     Message::TorqueControl(cmd) => {
    //         Commutator::set(TorqueControl::new(cmd.duration, cmd.currents).into())
    //     }
    //     Message::PosVelControl => Commutator::set(PosVelControl::new().into()),
    //     Message::PosVelCommand(cmd) => {
    //         PosVelControl::command(cmd);
    //     }
    //     Message::BeginStateStream(cmd) => {
    //         let frequency = cmd.frequency.max(1.);
    //         timer::periodic_callback(frequency, frequency / 10_000., || {
    //             if let Some(state) = SENSOR_STATE.read() {
    //                 fdcan::send_message(&state);
    //             };
    //         });
    //     }
    //     Message::EndStateStream => {
    //         timer::stop_periodic_callback();
    //     }
    //     _ => (),
    // });

    driver.add_message_handler(EnterTorqueControl::new());
    driver.add_message_handler(EnterPosVelControl::new());
    driver.add_message_handler(SetPosVel::new());
    driver.add_message_handler(DisableControlLoop::new());

    driver.listen();
}
