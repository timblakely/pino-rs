use cortex_m::{interrupt::InterruptNumber, peripheral::NVIC};
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
