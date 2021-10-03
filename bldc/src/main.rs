#![no_std]
#![no_main]
#![feature(unboxed_closures, fn_traits)]

use bldc::{
    comms::{fdcan, messages::Message},
    commutation::{
        calibrate_adc::CalibrateADCCmd,
        calibrate_e_zero::{CalibrateEZero, CalibrateEZeroCmd, EZeroMsg},
        Commutator,
    },
    driver,
};

#[cfg(feature = "panic-halt")]
use panic_halt as _;
#[cfg(feature = "panic-itm")]
use panic_itm as _; // you can put a breakpoint on `rust_begin_unwind` to catch panics

// TODO(blakely): Comment on all the stuff that happens before we actually get here...
#[cortex_m_rt::entry]
fn main() -> ! {
    // Acquire the driver.
    let mut driver = driver::take_hardware().configure_peripherals().calibrate();

    driver.on(Message::CalibrateEZero, |cmd: CalibrateEZeroCmd| {
        Commutator::set(CalibrateEZero::new(cmd.duration, cmd.currents, |_| {
            fdcan::send_message(&EZeroMsg {
                angle: 12.3,
                angle_raw: 456,
                e_angle: 0.789,
                e_raw: 1337.,
            })
        }))
    });

    driver.on(Message::CalibrateADC, |_frame: CalibrateADCCmd| {});

    driver.listen();

    // Listen for any incoming FDCAN messages.
    // driver.listen(|fdcan, message| {
    //     // We've received a message via the FDCAN.
    //     match Messages::unpack_fdcan(message) {
    //         Some(Messages::IdleCurrentSense(m)) => {
    //             Commutator::set(IdleCurrentSensor::new(m.duration, |measurement| {
    //                 fdcan.send_message(measurement);
    //             }))
    //         }
    //         Some(Messages::CalibrateADC(m)) => {
    //             Commutator::set(CalibrateADC::new(m.duration, |measurement| {
    //                 fdcan.send_message(measurement);
    //             }))
    //         }
    //         Some(Messages::IdleCurrentDistribution(m)) => {
    //             Commutator::set(IdleCurrentDistribution::new(
    //                 m.duration,
    //                 m.center_current,
    //                 m.current_range,
    //                 m.phase,
    //                 |bins| {
    //                     fdcan.send_message(&CurrentDistribution { bins });
    //                 },
    //             ))
    //         }
    //         Some(Messages::MeasureInductance(m)) => Commutator::set(MeasureInductance::new(
    //             m.duration,
    //             m.frequency,
    //             m.pwm_duty,
    //             m.sample_pwm_percent,
    //             |inductances| {
    //                 fdcan.send_message(&Inductances {
    //                     inductances: &inductances,
    //                 });
    //             },
    //         )),
    //         Some(Messages::MeasureResistance(m)) => Commutator::set(MeasureResistance::new(
    //             m.duration,
    //             m.target_voltage,
    //             m.phase,
    //             |measurement| {
    //                 fdcan.send_message(measurement);
    //             },
    //         )),
    //         Some(Messages::PhaseCurrentCommand(m)) => Commutator::set(PhaseCurrent::new(
    //             m.duration,
    //             m.target_current,
    //             m.phase,
    //             m.k,
    //             m.ki,
    //         )),
    //         Some(Messages::ReadEncoder(_)) => {
    //             Commutator::set(ReadEncoder::new(|results| fdcan.send_message(results)))
    //         }
    //         Some(Messages::CalibrateEZero(m)) => {
    //             Commutator::set(CalibrateEZero::new(m.duration, m.currents, |measurement| {
    //                 fdcan.send_message(measurement);
    //             }))
    //         }
    //         _ => (),
    //     };
    // });
    // loop {}
}
