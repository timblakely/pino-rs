use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, Ordering};
use cortex_m::{interrupt::InterruptNumber, peripheral::NVIC};
use stm32g4::stm32g474::Interrupt;
use third_party::m4vga_rs::util::armv7m::{disable_irq, enable_irq};
use third_party::m4vga_rs::util::spin_lock::{SpinLock, SpinLockGuard};

pub fn free_from<I: InterruptNumber, T: Send, F: FnOnce(SpinLockGuard<T>)>(
    irq: I,
    lock: &SpinLock<Option<T>>,
    f: F,
) {
    let enabled = NVIC::is_enabled(irq);
    disable_irq(irq);
    f(SpinLockGuard::map(
        lock.try_lock()
            .expect("Lock held prior to entering critical section"),
        |o| {
            o.as_mut()
                .expect("Critical section entered without HW available")
        },
    ));
    if enabled {
        enable_irq(irq);
    }
}

pub enum InterruptState {
    Active,
    Inactive,
}

pub fn in_interrupt(irq: impl InterruptNumber) -> InterruptState {
    match NVIC::is_active(irq) {
        true => InterruptState::Active,
        _ => InterruptState::Inactive,
    }
}

// Synchronization primitive that allows for locking of the contents while blocking an IRQ, allowing
// the data to be passed synchronously into the interrupt handler.
pub struct InterruptBLock<T> {
    irq: Interrupt,
    contents: UnsafeCell<T>,
    lock: AtomicBool,
}

#[derive(Copy, Clone, Debug)]
pub enum BLockError {
    Contended,
}

impl<T> InterruptBLock<T> {
    pub const fn new(irq: Interrupt, contents: T) -> InterruptBLock<T> {
        InterruptBLock {
            irq,
            contents: UnsafeCell::new(contents),
            lock: AtomicBool::new(false),
        }
    }

    pub fn try_lock(&self) -> Result<BLockGuard<T>, BLockError> {
        // Store whether the interrupt is enabled.
        let enabled = NVIC::is_enabled(self.irq);
        // Disable the interrupt first. If we're in an interrupt this has no effect.
        disable_irq(self.irq);
        if self.lock.swap(true, Ordering::Acquire) {
            // If there was already a true in there, lock was contended. Re-enable IRQ if it was
            // previously enabled.
            if enabled {
                enable_irq(self.irq);
            }
            Err(BLockError::Contended)
        } else {
            // Acquired lock! Can safely observe contents of this lock.
            Ok(BLockGuard {
                lock: &self.lock,
                contents: unsafe { &mut *self.contents.get() },
                irq: &self.irq,
                enabled,
            })
        }
    }

    pub fn lock(&self) -> BLockGuard<T> {
        loop {
            match self.try_lock() {
                Ok(guard) => return guard,
                _ => continue,
            }
        }
    }
}

pub struct BLockGuard<'a, T> {
    lock: &'a AtomicBool,
    contents: &'a mut T,
    enabled: bool,
    irq: &'a Interrupt,
}

impl<'a, T> core::ops::Deref for BLockGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.contents
    }
}

impl<'a, T> core::ops::DerefMut for BLockGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.contents
    }
}

impl<'a, T> Drop for BLockGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.store(false, Ordering::Release);
        if self.enabled {
            enable_irq(*self.irq);
        }
    }
}

unsafe impl<T: Send> Sync for InterruptBLock<T> {}
