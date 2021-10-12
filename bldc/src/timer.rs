extern crate alloc;

use alloc::boxed::Box;
#[cfg(not(feature = "host"))]
use num_traits::float::FloatCore;
use stm32g4::stm32g474::{self as device, interrupt};
use third_party::m4vga_rs::util::{
    armv7m::{clear_pending_irq, enable_irq},
    spin_lock::SpinLock,
    sync::acquire_hw,
};

use crate::{block_until, block_while, led, util::interrupts::block_interrupt};

#[derive(Debug, Clone, Copy)]
pub struct TimerConfig {
    pub prescalar: u16,
    pub arr: u16,
}

impl TimerConfig {
    pub fn frequency(&self, core_clock_hz: u32) -> f32 {
        core_clock_hz as f32 / (self.prescalar as f32 * self.arr as f32)
    }
}

#[derive(Debug)]
pub struct TimerInfo {
    pub frequency: f32,
    pub dt: f32,
    pub drift: f32,
}

#[derive(Debug)]
pub enum TimerCalculation {
    Approximate(TimerConfig),
    Exact(TimerConfig),
}

impl TimerCalculation {
    pub fn config(&self) -> TimerConfig {
        match self {
            Self::Approximate(x) => *x,
            Self::Exact(x) => *x,
        }
    }
}

pub fn timer_info(core_clock_hz: u32, config: &TimerConfig) -> TimerInfo {
    let timer_frequency = config.frequency(core_clock_hz);
    let drift = (1. / (core_clock_hz as f32 / (config.prescalar * config.arr) as f32)
        - 1. / (timer_frequency as f32))
        * 1e9;
    TimerInfo {
        frequency: timer_frequency,
        dt: 1. / timer_frequency,
        drift,
    }
}

pub fn iteratively_calculate_timer_config(
    core_clock_hz: u32,
    desired_frequency: f32,
    tolerance: f32,
) -> Option<TimerCalculation> {
    let mut prescalar: u16 = 1;
    let mut diff = f32::MAX;
    let mut closest_arr = u16::MAX;
    let mut closest_prescalar: u16 = 1;
    loop {
        let arr = ((core_clock_hz as f32) / (desired_frequency * (prescalar as f32))).ceil() as u32;
        let config = TimerConfig {
            arr: arr as u16,
            prescalar,
        };
        let current_diff = (config.frequency(core_clock_hz) - desired_frequency).abs();
        if arr < (1 << 16) {
            if current_diff < diff {
                closest_prescalar = prescalar;
                closest_arr = arr as u16;
                diff = current_diff;
            }
            if current_diff == 0. {
                return Some(TimerCalculation::Exact(TimerConfig {
                    prescalar: closest_prescalar,
                    arr: closest_arr,
                }));
            }
            if (current_diff as f32) < tolerance {
                // Approximate match
                return Some(TimerCalculation::Approximate(TimerConfig {
                    prescalar: closest_prescalar,
                    arr: closest_arr,
                }));
            }
        }
        prescalar += 1;
        if prescalar == u16::MAX {
            return None;
        }
    }
}

struct Scheduler {
    pub tim2: device::TIM2,
    pub callback: Option<Box<dyn FnMut()>>,
}
unsafe impl Send for Scheduler {}

static SCHEDULER: SpinLock<Option<Scheduler>> = SpinLock::new(None);

// Configure TIM2 for use as an optional scheduled callback.
pub fn donate_hardware_for_scheduler(tim2: device::TIM2) {
    enable_irq(device::Interrupt::TIM2);
    // Stop the timer if it's running for some reason.
    tim2.cr1.modify(|_, w| w.cen().disabled());
    block_until!(tim2.cr1.read().cen().bit_is_clear());
    // Up counting, edge-aligned mode.
    tim2.cr1.modify(|_, w| w.dir().up().cms().edge_aligned());
    // Enable interrupt on Update
    tim2.dier.modify(|_, w| w.uie().enabled());
    // But _don't_ start it now in case we don't need it. Just store it for later use
    *SCHEDULER.lock() = Some(Scheduler {
        tim2,
        callback: None,
    });
}

pub fn periodic_callback(frequency: f32, tolerance: f32, callback: impl FnMut()) {
    block_interrupt(device::Interrupt::TIM2, &SCHEDULER, |mut scheduler| {
        let boxed: Box<dyn FnMut()> = Box::new(callback);
        // Safety: in order to store something in a global static, it _must_ have a 'static lifetime,
        // even if it's stored on the heap (???). So this transmutes it to a static lifetime.
        scheduler.callback = unsafe { core::mem::transmute(Some(boxed)) };

        // Configure the timer now.
        let timer_config = iteratively_calculate_timer_config(170_000_000, frequency, tolerance)
            .expect("Unable to find appropriate timing")
            .config();
        let tim2 = &mut scheduler.tim2;
        // We subtract one here since the PSC field of this register is actually `prescalar - 1`
        tim2.psc.write(|w| w.psc().bits(timer_config.prescalar - 1));
        // Safety: Upstream PAC needs bounds. TIM2 is a 32 bit timer, and we're setting at max 16
        // bits here.
        tim2.arr
            .write(|w| unsafe { w.bits(timer_config.arr as u32) });
        // Enable the timer
        tim2.cr1.modify(|_, w| w.cen().enabled());
    });
}

pub fn stop_periodic_callback() {
    block_interrupt(device::Interrupt::TIM2, &SCHEDULER, |scheduler| {
        scheduler.tim2.cr1.modify(|_, w| w.cen().disabled());
    });
}

// Interrupt handler triggered by TIM2's update event.
#[interrupt]
fn TIM2() {
    // Clear the IRQ so it doesn't immediately fire again.
    clear_pending_irq(device::Interrupt::TIM2);
    led::Led::<led::Green>::on_while(|| {
        cortex_m::asm::nop();
        cortex_m::asm::nop();
        cortex_m::asm::nop();
        cortex_m::asm::nop();
        cortex_m::asm::nop();
        cortex_m::asm::nop();
        cortex_m::asm::nop();
        cortex_m::asm::nop();
    });

    let mut scheduler = acquire_hw(&SCHEDULER);

    scheduler.tim2.sr.modify(|_, w| w.uif().clear_bit());
    if let Some(ref mut callback) = scheduler.callback {
        callback();
    }
}
