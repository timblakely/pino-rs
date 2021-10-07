//! Transaction/Buffer-Based Primitive
//!
//! A type of synchronization primitive used to share a common state where the reader can preempt
//! the writer. Particularly useful when you've got an interrupt that you cannot interrupt with a
//! critical section e.g. a control loop. This effectively provides eventual consistency on the
//! reads.

use core::{
    ops::{Deref, DerefMut},
    ptr::NonNull,
    sync::atomic::{AtomicUsize, Ordering},
};

pub struct BufferedState<T: Sized + Copy> {
    current: AtomicUsize,
    value: [T; 2],
}

impl<'a, T: Sized + Copy> BufferedState<T> {
    // Creates a BufferedState
    pub fn new(initial_state: T) -> Self {
        BufferedState {
            current: AtomicUsize::new(0),
            value: [initial_state.clone(), initial_state.clone()],
        }
    }

    pub fn split(&mut self) -> (StateReader<T>, StateWriter<T>) {
        (
            StateReader {
                state: NonNull::new(self).expect("Passed a null ptr"),
            },
            StateWriter {
                state: NonNull::new(self).expect("Passed a null ptr"),
            },
        )
    }
}

pub struct StateReader<T: Copy> {
    state: NonNull<BufferedState<T>>,
}

impl<T: Copy> StateReader<T> {
    pub fn read(&self) -> &T {
        // Safety: enforced to be non-null by NonNull
        let state = unsafe { self.state.as_ref() };
        let reading_state = state.current.load(Ordering::Acquire);
        match reading_state {
            0 => &state.value[0],
            _ => &state.value[1],
        }
    }
}

unsafe impl<T: Copy> Send for StateReader<T> {}

pub struct StateWriter<T: Copy> {
    state: NonNull<BufferedState<T>>,
}

// TODO(blakely): Fix this so that it's actually Send with proper locking.
unsafe impl<T: Copy> Send for StateWriter<T> {}

impl<T: Copy> StateWriter<T> {
    pub fn update(&mut self) -> StateGuard<T> {
        // Safety: enforced to be non-null by NonNull
        let state = unsafe { self.state.as_mut() };

        StateGuard::new(&mut state.value, &mut state.current)
    }
}

pub struct StateGuard<'a, T: Copy> {
    data: &'a mut [T; 2],
    current: &'a mut AtomicUsize,
    target: usize,
    other: usize,
}

impl<'a, T: Copy> StateGuard<'a, T> {
    fn new(data: &'a mut [T; 2], current: &'a mut AtomicUsize) -> StateGuard<'a, T> {
        // Identify which buffer is safe for writing to.
        let (target, other) = match current.load(Ordering::Acquire) {
            0 => (1, 0),
            _ => (0, 1),
        };
        StateGuard {
            data,
            current,
            target,
            other,
        }
    }

    pub fn other(&self) -> &T {
        &self.data[self.other]
    }
}

impl<'a, T: Copy> Deref for StateGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data[self.target]
    }
}

impl<'a, T: Copy> DerefMut for StateGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // Guaranteed to be not-null by NotNull, and lifetime guarded by 'a.
        &mut self.data[self.target]
    }
}

impl<'a, T: Copy> Drop for StateGuard<'a, T> {
    fn drop(&mut self) {
        self.current.store(self.target, Ordering::Relaxed);
    }
}