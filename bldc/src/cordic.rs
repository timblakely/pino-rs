use core::marker::PhantomData;

use core::f32::consts::PI;
use fixed::types::I1F31;
use stm32g4::stm32g474::{self as device};
use third_party::m4vga_rs::util::spin_lock::{SpinLock, SpinLockGuard};

use crate::{block_until, block_while};

const TWO_PI: f32 = 2. * PI;

// Functional API around CORDIC hardware.

pub struct Cordic {
    device: SpinLock<device::CORDIC>,
}

pub struct CordicProcessing<'a, const N: usize> {
    cordic: SpinLockGuard<'a, device::CORDIC>,
    _marker: PhantomData<&'a ()>,
}

impl<'a, const N: usize> CordicProcessing<'a, N> {
    // Blocks until the result is ready
    pub fn get_result(self) -> [f32; N] {
        let cordic = self.cordic;
        block_until!(cordic.csr.read().rrdy().is_ready());
        let mut result: [f32; N] = [0.; N];
        for i in 0..N {
            result[i] = I1F31::from_bits(cordic.rdata.read().bits() as i32).to_num();
        }
        result
    }
}

fn to_q1_31(theta: f32) -> I1F31 {
    // Normalize to [-pi,pi] if necessary
    let normalized_radians = match theta {
        t if t >= PI => t - (((t + PI) / TWO_PI) as i32) as f32 * TWO_PI,
        t if t < PI => t - (((t - PI) / TWO_PI) as i32) as f32 * TWO_PI,
        t => t,
    };
    let normalized_linear = normalized_radians / PI;
    I1F31::from_num(normalized_linear.clamp(-1., 1. - f32::EPSILON))
}

impl Cordic {
    pub fn new(cordic: device::CORDIC, iterations: u16) -> Cordic {
        // CORDIC runs at 4x core clock speed.
        let precision: u8 = (iterations / 4) as u8;
        // Set the precision and argument/result size now, wait for configuraiton later.
        // Safety: yet another SVD range missing. Valid ranges for precision is 1-15
        cordic.csr.modify(|_, w| unsafe {
            w.precision()
                .bits(precision)
                .ressize()
                .bits32()
                .argsize()
                .bits32()
        });
        Cordic {
            device: SpinLock::new(cordic),
        }
    }

    pub fn cos_sin<'a>(&'a mut self, theta: f32) -> CordicProcessing<'a, 2> {
        let cordic = self
            .device
            .try_lock()
            .expect("Cordic lock held, already processing");
        // Configure for cosine functionality.
        cordic
            .csr
            .modify(|_, w| w.func().cosine().nres().num2().nargs().num1());
        let q1_31 = to_q1_31(theta);
        cordic
            .wdata
            .write(|w| unsafe { w.bits(q1_31.to_bits() as u32) });
        CordicProcessing {
            cordic,
            _marker: PhantomData,
        }
    }
}
