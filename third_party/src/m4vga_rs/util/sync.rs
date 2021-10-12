use super::spin_lock::{SpinLock, SpinLockGuard};

/// Pattern for acquiring hardware resources loaned to an ISR in a static.
///
/// # Panics
///
/// If the `SpinLock` is locked when this is called. This would imply:
///
/// 1. that the IRQ got enabled too early, while the hardware is being
///    provisioned;
/// 2. That two ISRs are attempting to use the hardware without coordination.
/// 3. That a previous invocation of an ISR leaked the lock guard.
///
/// Also: if this is called before hardware is provisioned, implying that the
/// IRQ was enabled too early.
pub fn acquire_hw<T: Send>(lock: &SpinLock<Option<T>>) -> SpinLockGuard<T> {
    SpinLockGuard::map(lock.try_lock().expect("HW lock held at ISR"), |o| {
        o.as_mut().expect("ISR fired without HW available")
    })
}
