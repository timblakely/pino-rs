use core::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::atomic::{fence, AtomicUsize, Ordering},
};
use third_party::m4vga_rs::util::rw_lock::{GuardMut, ReadWriteLock, TryLockMutError};

pub struct SeqLockGuard<'a, T: Copy> {
    _guard: GuardMut<'a, ()>,
    seqlock: &'a SeqLock<T>,
    seq: usize,
}

impl<'a, T: Copy> Deref for SeqLockGuard<'a, T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &Self::Target {
        // Safety: dereferencing a raw pointer is inherently unsafe. But the SeqLock's UnsafeCell
        // has been initialized first thing in `::new()`.
        unsafe { &*self.seqlock.value.get() }
    }
}

impl<'a, T: Copy> DerefMut for SeqLockGuard<'a, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        // Safety: dereferencing a raw pointer is inherently unsafe. But the SeqLock's UnsafeCell
        // has been initialized first thing in `::new()`.
        unsafe { &mut *self.seqlock.value.get() }
    }
}

// On drop, call the wrapped SeqLock with the original sequence number.
impl<'a, T: Copy> Drop for SeqLockGuard<'a, T> {
    fn drop(&mut self) {
        self.seqlock.finish_write(self.seq);
    }
}

pub struct SeqLock<T: Copy> {
    sequence: AtomicUsize,
    value: UnsafeCell<T>,
    lock: ReadWriteLock<()>,
}

// Read-optimized lock that allows writing to preempt reading. Reading will block until there are no
// threads writing to the lock. Only a single writer can write to the contents of a SeqLock at a
// time as it's guarded by a ReadWriteLock internally.
impl<T: Copy> SeqLock<T> {
    pub fn new(initial_state: T) -> Self {
        SeqLock {
            sequence: AtomicUsize::new(0),
            value: UnsafeCell::new(initial_state),
            lock: ReadWriteLock::new(()),
        }
    }

    // Acquire the lock by reading the seqeuence, making a copy, and reading the sequence again. If
    // the sequence is the same, that means the value can be assumed to be safe. If it isn't,
    // another context has written to and potentially changed the protected value.
    pub fn read(&self) -> T {
        loop {
            // Load the sequence, making sure we order it by Acquire so it's done before the read.
            let seq1 = self.sequence.load(Ordering::Acquire);

            // Optimization: if the sequence number is odd, a writer is holding the lock. Instead of
            // copying the value and wasting cycles, restart the loop and re-acquire the sequence
            // number as fast as possible.
            if seq1 & 1 != 0 {
                continue;
            }

            // The data may be concurrently modified by a writer, so make sure the read is volatile
            // (non-caching and not reordered).
            // Safety: `read_volatile` requires three conditions:
            // * `src` must be [valid] for reads.
            // * `src` must be properly aligned.
            // * `src` must point to a properly initialized value of type `T`.
            // The UnsafeCell is valid for reads, alignment is taken care of by the struct, and the
            // initial state was loaded in the constructor.
            let stored_value = unsafe { core::ptr::read_volatile(self.value.get()) };

            // Ensure that the second read is not performed out-of-order.
            fence(Ordering::Acquire);

            // If the sequence number is the same, a writer has not modified the data and it's safe
            // to return.
            if seq1 == self.sequence.load(Ordering::Relaxed) {
                return stored_value;
            }

            // The data was modified in the meantime by another thread. Need to try again...
        }
    }

    // Increase the sequence number to indicate to readers that we are in the process of writing.
    // Can only be called with a proper RwLock guard, indicating that it has been locked.
    fn with_guard<'a>(&'a self, guard: GuardMut<'a, ()>) -> SeqLockGuard<'a, T> {
        // First thing: get the old value and increment the sequence number.
        let seq = self.sequence.fetch_add(1, Ordering::Acquire);

        // Create a guard that, when dropped, will call `SeqLock.finish_write`
        SeqLockGuard {
            _guard: guard,
            seq,
            seqlock: self,
        }
    }

    // Get the locked value by reference, mutably. Note that this does not require locking because
    // the SeqLock is borrowed _mutably_, which means no other references can be held to it, and
    // thus no locks exist on it.
    pub fn get_mut(&mut self) -> &mut T {
        self.value.get_mut()
    }

    // Lock the SeqLock for writing. Will block until lock is available.
    pub fn lock_write(&self) -> SeqLockGuard<T> {
        self.with_guard(self.lock.lock_mut())
    }

    // Attempt to lock the SeqLock for writing. If unsucessful, will return immediately.
    pub fn try_lock_write(&self) -> Result<SeqLockGuard<T>, TryLockMutError> {
        self.lock.try_lock_mut().map(|g| self.with_guard(g))
    }

    // Called by SeqLockGuard during Drop.
    pub fn finish_write(&self, sequence: usize) {
        self.sequence.store(sequence, Ordering::Relaxed);
    }
}

// With the above locking guarantees, we can make this both Send and Sync.
unsafe impl<T: Copy + Send> Send for SeqLock<T> {}
unsafe impl<T: Copy + Send> Sync for SeqLock<T> {}
