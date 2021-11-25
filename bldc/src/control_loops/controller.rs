use super::calibrate_adc::CalibrateADC;
use super::pos_vel_control::PositionVelocity;
use super::torque_control::TorqueControl;
use super::{ControlHardware, SensorState};
use crate::util::interrupts::block_interrupt;
use crate::util::seq_lock::SeqLock;
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering;
use enum_dispatch::enum_dispatch;
use lazy_static::lazy_static;
use stm32g4::stm32g474 as device;
use third_party::m4vga_rs::util::spin_lock::SpinLock;

// TODO(blakely): move to mod.rs
#[enum_dispatch(Commutate)]
pub enum ControlLoop {
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
    ) -> LoopState;
    fn finished(&mut self);
}

pub struct InterruptData {
    pub control_loop: Option<ControlLoop>,
    pub hw: ControlHardware,
}

pub static COMMUTATING: AtomicBool = AtomicBool::new(false);
pub static INTERRUPT_SHARED: SpinLock<Option<InterruptData>> = SpinLock::new(None);
lazy_static! {
    pub static ref SENSOR_STATE: SeqLock<Option<SensorState>> = SeqLock::new(None);
}

pub enum LoopState {
    Running,
    Finished,
}

pub struct Controller {}

impl Controller {
    pub fn new() -> Controller {
        Controller {}
    }

    pub fn set_loop<C>(&self, control_loop: C)
    where
        C: Into<ControlLoop>,
    {
        block_interrupt(device::interrupt::ADC1_2, &INTERRUPT_SHARED, |mut vars| {
            vars.control_loop = Some(control_loop.into());
        });
    }

    pub fn enable_loop(&self) {
        block_interrupt(
            device::interrupt::ADC1_2,
            &INTERRUPT_SHARED,
            |mut control_vars| {
                COMMUTATING.store(true, Ordering::Relaxed);
                control_vars.hw.pwm.enable_loop();
            },
        );
    }

    pub fn is_enabled(&self) -> bool {
        COMMUTATING.load(Ordering::Acquire)
    }

    // TODO(blakely): Don't disable the loop until currents have settled down low enough.
    pub fn disable_loop(&self) {
        block_interrupt(
            device::interrupt::ADC1_2,
            &INTERRUPT_SHARED,
            |mut control_vars| {
                COMMUTATING.store(false, Ordering::Relaxed);
                control_vars.hw.pwm.disable_loop();
            },
        );
    }

    pub fn donate_hardware(&self, hw: ControlHardware) {
        *INTERRUPT_SHARED
            .try_lock()
            .expect("Lock held while trying to donate hardware") = Some(InterruptData {
            control_loop: None,
            hw,
        });
    }
}
