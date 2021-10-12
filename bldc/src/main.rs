#![cfg_attr(not(test), no_std)]
#![no_main]

use bldc::comms::messages::Message;
use bldc::{
    comms::fdcan::{self, IncomingFdcanFrame},
    commutation::{
        calibrate_e_zero::{CalibrateEZeroCmd, EZeroMsg},
        pos_vel_control::{PosVelCommand, PosVelControl, PosVelMode},
        torque_control::{TorqueControl, TorqueControlCmd},
        Commutator, SENSOR_STATE,
    },
    driver, timer,
};

#[cfg(feature = "panic-halt")]
use panic_halt as _;
#[cfg(feature = "panic-itm")]
use panic_itm as _; // you can put a breakpoint on `rust_begin_unwind` to catch panics

struct StartStreamCmd {
    pub frequency: f32,
}
impl IncomingFdcanFrame for StartStreamCmd {
    fn unpack(msg: fdcan::FdcanMessage) -> Self {
        let buffer = msg.data;
        StartStreamCmd {
            frequency: f32::from_bits(buffer[0]),
        }
    }
}

struct StopStreamCmd {}
impl IncomingFdcanFrame for StopStreamCmd {
    fn unpack(_: fdcan::FdcanMessage) -> Self {
        StopStreamCmd {}
    }
}

// TODO(blakely): Comment on all the stuff that happens before we actually get here...
#[cortex_m_rt::entry]
fn main() -> ! {
    // Acquire the driver.
    let mut driver = driver::take_hardware().configure_peripherals().calibrate();

    driver.on(Message::CalibrateEZero, |_: CalibrateEZeroCmd| {
        fdcan::send_message(&EZeroMsg {
            angle: 123.,
            angle_raw: 456,
            e_angle: 789.,
            e_raw: 1337.,
        });
    });

    driver.on(Message::TorqueControl, |cmd: TorqueControlCmd| {
        Commutator::set(TorqueControl::new(cmd.duration, cmd.currents))
    });

    driver.on(Message::PosVelControl, |_: PosVelMode| {
        Commutator::set(PosVelControl::new())
    });

    driver.on(Message::PosVelCommand, |cmd: PosVelCommand| {
        PosVelControl::command(cmd);
    });

    driver.on(Message::BeginStateStream, |cmd: StartStreamCmd| {
        let frequency = cmd.frequency.max(1.);
        timer::periodic_callback(frequency, frequency / 10_000., || {
            if let Some(state) = SENSOR_STATE.read() {
                fdcan::send_message(&state);
            };
        });
    });

    driver.on(Message::EndStateStream, |_cmd: StopStreamCmd| {
        timer::stop_periodic_callback();
    });

    driver.listen();
}
