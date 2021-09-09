use crate::{
    current_sensing::{self, CurrentMeasurement, CurrentSensor},
    ic::ma702::{Ma702, Streaming},
    util::interrupts::InterruptBLock,
};
use stm32g4::stm32g474::{self as device, interrupt};
extern crate alloc;
use alloc::boxed::Box;
use third_party::m4vga_rs::util::armv7m::clear_pending_irq;

pub struct ControlHardware {
    pub current_sensor: CurrentSensor<current_sensing::Ready>,
    pub tim1: device::TIM1,
    pub ma702: Ma702<Streaming>,
}

// TODO(blakely): Wrap the peripherals in some slightly higher-level abstractions.
pub struct ControlLoopVars {
    pub control_loop: Option<Box<dyn ControlLoop>>,
    pub hw: ControlHardware,
}

pub static CONTROL_LOOP: InterruptBLock<Option<ControlLoopVars>> =
    InterruptBLock::new(device::interrupt::ADC1_2, None);

pub struct Commutator {}

impl Commutator {
    pub fn set<'a>(commutator: impl ControlLoop + 'a) {
        let boxed: Box<dyn ControlLoop> = Box::new(commutator);
        match *CONTROL_LOOP.lock() {
            Some(ref mut v) => (*v).control_loop = unsafe { core::mem::transmute(Some(boxed)) },
            None => panic!("Loop variables not set"),
        };
    }
}

pub enum LoopState {
    Running,
    Finished,
}

// Trait that any control loops need to implement.
pub trait ControlLoop: Send {
    fn commutate(&mut self, hardware: &mut ControlHardware) -> LoopState;
    fn finished(&mut self) {}
}

// During commutation, no PWM is performed. The current is sampled once at each loop for a given
// duration then averaged across all samples.
pub struct IdleCurrentSensor<'a> {
    total_counts: u32,
    loop_count: u32,
    sample: CurrentMeasurement,
    callback: Box<dyn for<'r> FnMut(&'r CurrentMeasurement) + 'a + Send>,
}

impl<'a> IdleCurrentSensor<'a> {
    pub fn new(
        duration: f32,
        callback: impl for<'r> FnMut(&'r CurrentMeasurement) + 'a + Send,
    ) -> IdleCurrentSensor<'a> {
        IdleCurrentSensor {
            total_counts: (40_000 as f32 * duration) as u32,
            loop_count: 0,
            sample: CurrentMeasurement::new(),
            callback: Box::new(callback),
        }
    }
}

impl<'a> ControlLoop for IdleCurrentSensor<'a> {
    fn commutate(&mut self, hardware: &mut ControlHardware) -> LoopState {
        self.loop_count += 1;
        let current_sensor = &hardware.current_sensor;
        self.sample += current_sensor.sample();

        match self.loop_count {
            x if x >= self.total_counts => LoopState::Finished,
            _ => LoopState::Running,
        }
    }

    fn finished(&mut self) {
        self.sample /= self.loop_count;
        (self.callback)(&self.sample);
    }
}

enum Phase {
    A,
    B,
    C,
}

pub struct IdleCurrentDistribution<'a> {
    total_counts: u32,
    loop_count: u32,
    bins: [u32; 16],
    current_min: f32,
    current_binsize: f32,
    phase: Phase,
    callback: Box<dyn for<'r> FnMut(&'r [u32; 16]) + 'a + Send>,
}

impl<'a> IdleCurrentDistribution<'a> {
    pub fn new(
        duration: f32,
        center: f32,
        range: f32,
        phase: u8,
        callback: impl for<'r> FnMut(&'r [u32; 16]) + 'a + Send,
    ) -> IdleCurrentDistribution<'a> {
        let current_min = center - range;
        let current_binsize = range * 2. / 16.;
        let phase = match phase {
            0 => Phase::A,
            1 => Phase::B,
            _ => Phase::C,
        };
        IdleCurrentDistribution {
            total_counts: (40_000 as f32 * duration) as u32,
            loop_count: 0,
            bins: [0; 16],
            current_min,
            current_binsize,
            phase,
            callback: Box::new(callback),
        }
    }
}

impl<'a> ControlLoop for IdleCurrentDistribution<'a> {
    fn commutate(&mut self, hardware: &mut ControlHardware) -> LoopState {
        self.loop_count += 1;
        let current_sensor = &hardware.current_sensor;
        let sample = current_sensor.sample();

        let current_value = match self.phase {
            Phase::A => sample.phase_a,
            Phase::B => sample.phase_b,
            Phase::C => sample.phase_c,
        };

        let bin_index = ((current_value - self.current_min) / self.current_binsize) as usize;
        let bin_index = bin_index.max(0).min(self.bins.len() - 1);
        self.bins[bin_index] += 1;

        match self.loop_count {
            x if x >= self.total_counts => LoopState::Finished,
            _ => LoopState::Running,
        }
    }

    fn finished(&mut self) {
        (self.callback)(&self.bins);
    }
}

/////

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
            commutator.finished();
            loop_vars.control_loop = None;
        }
        _ => return,
    }
}
