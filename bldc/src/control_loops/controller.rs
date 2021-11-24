use enum_dispatch::enum_dispatch;

use crate::util::interrupts::block_interrupt;

use super::calibrate_adc::CalibrateADC;
use super::pos_vel_control::PositionVelocity;
use super::torque_control::TorqueControl;
use super::{CommutationState, ControlHardware, SensorState, COMMUTATING, COMMUTATION_STATE};
use core::sync::atomic::Ordering;
use stm32g4::stm32g474 as device;

pub enum ControlLoop {
    Running,
    Finished,
}

#[enum_dispatch(Commutate)]
pub enum Controller {
    CalibrateADC,
    TorqueControl,
    PositionVelocity,
}

// Trait that any control loops need to implement.
#[enum_dispatch]
pub trait Commutate: Send {
    fn commutate(
        &mut self,
        sensor_state: &SensorState,
        hardware: &mut ControlHardware,
    ) -> ControlLoop;
    fn finished(&mut self);
}

impl Controller {
    pub fn new() -> Controller {
        Controller {}
    }

    pub fn donate_hardware(hw: ControlHardware) {
        *COMMUTATION_STATE
            .try_lock()
            .expect("Lock held while trying to donate hardware") = Some(CommutationState {
            commutator: None,
            hw,
        });
    }

    pub fn set<C>(commutator: C)
    where
        C: Into<Controller>,
    {
        block_interrupt(device::interrupt::ADC1_2, &COMMUTATION_STATE, |mut vars| {
            vars.commutator = Some(commutator.into());
        });
    }

    pub fn enable_loop() {
        block_interrupt(
            device::interrupt::ADC1_2,
            &COMMUTATION_STATE,
            |mut control_vars| {
                COMMUTATING.store(true, Ordering::Relaxed);
                control_vars.hw.pwm.enable_loop();
            },
        );
    }

    pub fn is_enabled() -> bool {
        COMMUTATING.load(Ordering::Acquire)
    }

    // TODO(blakely): Don't disable the loop until currents have settled down low enough.
    pub fn disable_loop() {
        block_interrupt(
            device::interrupt::ADC1_2,
            &COMMUTATION_STATE,
            |mut control_vars| {
                COMMUTATING.store(false, Ordering::Relaxed);
                control_vars.hw.pwm.disable_loop();
            },
        );
    }
}
