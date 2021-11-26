use super::calibrate_adc::CalibrateADC;
use super::pos_vel_control::PositionVelocity;
use super::torque_control::TorqueControl;
use super::{ControlHardware, SensorState};
use crate::util::interrupts::block_interrupt;
use crate::util::seq_lock::SeqLock;
use enum_dispatch::enum_dispatch;
use lazy_static::lazy_static;
use stm32g4::stm32g474 as device;
use third_party::m4vga_rs::util::armv7m::{disable_irq, enable_irq};
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
        loop_state: LoopState,
        sensor_state: &SensorState,
        hardware: &mut ControlHardware,
    ) -> LoopState;
    fn finished(&mut self);
}

pub struct InterruptData {
    pub control_loop: Option<ControlLoop>,
    pub hw: ControlHardware,
}

pub static INTERRUPT_SHARED: SpinLock<Option<InterruptData>> = SpinLock::new(None);
lazy_static! {
    pub static ref SENSOR_STATE: SeqLock<Option<SensorState>> = SeqLock::new(None);
}

#[derive(Clone, Copy)]
pub enum LoopState {
    Running,
    Shutdown,
    Idle,
}
lazy_static! {
    pub static ref LOOP_STATE: SeqLock<LoopState> = SeqLock::new(LoopState::Idle);
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
                *LOOP_STATE.lock_write() = LoopState::Running;
                control_vars.hw.pwm.enable_loop();
            },
        );
    }

    pub fn is_enabled(&self) -> bool {
        match LOOP_STATE.read() {
            LoopState::Idle => false,
            _ => true,
        }
    }

    pub fn disable_loop(&self) {
        // If we're not enabled, don't do anything.
        if !self.is_enabled() {
            return;
        }

        // In the event we're doing high speed commutation, give the underlying loop some time to
        // slow the motor down before shutting off the power to it. The loop may be running at the
        // moment, so we should only block the loop interrupt very briefly while signaling a
        // shutdown and then wait for it to be idle.
        disable_irq(device::interrupt::ADC1_2);
        // TODO(blakely): Maybe this should be a typestate transition struct?
        // Now that the IRQ is disabled, signal that we would like the loop to shut down.
        *LOOP_STATE.lock_write() = LoopState::Shutdown;
        // Reenable the interrupt, then wait for the loop to idle.
        enable_irq(device::interrupt::ADC1_2);

        loop {
            if let LoopState::Idle = LOOP_STATE.read() {
                break;
            }
        }

        block_interrupt(
            device::interrupt::ADC1_2,
            &INTERRUPT_SHARED,
            |mut control_vars| {
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
