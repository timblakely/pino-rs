#![no_std]
#![no_main]
#![feature(unboxed_closures, fn_traits)]

use bldc::{
    comms::messages::{CurrentDistribution, Inductances, Messages, OutgoingFdcanFrame},
    commutation::{
        calibrate_e_zero::CalibrateEZero,
        field_oriented_control::{DQCurrents, FieldOrientedControl},
        measure_inductance::MeasureInductance,
        measure_resistance::MeasureResistance,
        phase_current::PhaseCurrent,
        read_encoder::ReadEncoder,
        CalibrateADC, Commutator, IdleCurrentDistribution, IdleCurrentSensor,
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
    let driver = driver::take_hardware().configure_peripherals().calibrate();

    // Commutator::set(FieldOrientedControl::new(DQCurrents { q: 1., d: 0. }));
    // Commutator::set(FieldOrientedControl::new(DQCurrents { q: 0.0, d: 1.0 }));

    // Listen for any incoming FDCAN messages.
    driver.listen(|fdcan, message| {
        // We've received a message via the FDCAN.
        match Messages::unpack_fdcan(message) {
            Some(Messages::IdleCurrentSense(m)) => {
                Commutator::set(IdleCurrentSensor::new(m.duration, |measurement| {
                    fdcan.send_message(measurement.pack());
                }))
            }
            Some(Messages::CalibrateADC(m)) => {
                Commutator::set(CalibrateADC::new(m.duration, |measurement| {
                    fdcan.send_message(measurement.pack());
                }))
            }
            Some(Messages::IdleCurrentDistribution(m)) => {
                Commutator::set(IdleCurrentDistribution::new(
                    m.duration,
                    m.center_current,
                    m.current_range,
                    m.phase,
                    |bins| {
                        fdcan.send_message(CurrentDistribution { bins }.pack());
                    },
                ))
            }
            Some(Messages::MeasureInductance(m)) => Commutator::set(MeasureInductance::new(
                m.duration,
                m.frequency,
                m.pwm_duty,
                m.sample_pwm_percent,
                |inductances| {
                    fdcan.send_message(
                        Inductances {
                            inductances: &inductances,
                        }
                        .pack(),
                    );
                },
            )),
            Some(Messages::MeasureResistance(m)) => Commutator::set(MeasureResistance::new(
                m.duration,
                m.target_voltage,
                m.phase,
                |measurement| {
                    fdcan.send_message(measurement.pack());
                },
            )),
            Some(Messages::PhaseCurrentCommand(m)) => Commutator::set(PhaseCurrent::new(
                m.duration,
                m.target_current,
                m.phase,
                m.k,
                m.ki,
            )),
            Some(Messages::ReadEncoder(_)) => Commutator::set(ReadEncoder::new(|results| {
                fdcan.send_message(results.pack())
            })),
            Some(Messages::CalibrateEZero(m)) => {
                Commutator::set(CalibrateEZero::new(m.duration, m.currents, |measurement| {
                    fdcan.send_message(measurement.pack());
                }))
            }
            _ => (),
        };
    });
    loop {}
}
