//! Transaction/Buffer-Based Primitive
//!
//! A type of synchronization primitive used to share a common state where the reader can preempt
//! the writer. Particularly useful when you've got an interrupt that you cannot interrupt with a
//! critical section e.g. a control loop. This effectively provides eventual consistency on the
//! reads.

use core::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
    ptr::NonNull,
    sync::atomic::{AtomicUsize, Ordering},
};

pub struct BufferedState<'a, T: Sized + Copy> {
    current: AtomicUsize,
    value: [T; 2],
    _life: PhantomData<&'a ()>,
}

impl<'a, T: Sized + Copy> BufferedState<'a, T> {
    // Creates a BufferedState
    pub fn new(initial_state: T) -> Self {
        BufferedState {
            current: AtomicUsize::new(0),
            value: [initial_state.clone(), initial_state.clone()],
            _life: PhantomData,
        }
    }

    pub fn split(&mut self) -> (StateReader<'a, T>, StateWriter<'a, T>) {
        (
            StateReader {
                state: unsafe { NonNull::new_unchecked(self) },
                _life: PhantomData,
            },
            StateWriter {
                state: unsafe { NonNull::new_unchecked(self) },
                _life: PhantomData,
            },
        )
    }
}

pub struct StateReader<'a, T: Copy> {
    state: NonNull<BufferedState<'a, T>>,
    _life: PhantomData<&'a ()>,
}

impl<'a, T: Copy> StateReader<'a, T> {
    pub fn read<'t>(&'t self) -> &'t T {
        // Safety: enforced to be non-null by NonNull
        let state = unsafe { self.state.as_ref() };
        match state.current.load(Ordering::Acquire) {
            0 => &state.value[0],
            _ => &state.value[1],
        }
    }
}

unsafe impl<'a, T: Copy> Send for StateReader<'a, T> {}

pub struct StateWriter<'a, T: Copy> {
    state: NonNull<BufferedState<'a, T>>,
    _life: PhantomData<&'a ()>,
}

impl<'a, T: Copy> StateWriter<'a, T> {
    pub fn update(&mut self) -> StateGuard<'a, T> {
        // Safety: enforced to be non-null by NonNull
        let state = unsafe { self.state.as_mut() };

        // Identify which buffer is safe for writing to.
        let target = match state.current.load(Ordering::Acquire) {
            0 => 1,
            _ => 0,
        };

        StateGuard {
            data: &mut state.value[target],
            current: &mut state.current,
            target,
        }
    }
}

pub struct StateGuard<'a, T: Copy> {
    data: &'a mut T,
    current: &'a mut AtomicUsize,
    target: usize,
}

impl<'a, T: Copy> Deref for StateGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.data
    }
}

impl<'a, T: Copy> DerefMut for StateGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // Guaranteed to be not-null by NotNull, and lifetime guarded by 'a.
        self.data
    }
}

impl<'a, T: Copy> Drop for StateGuard<'a, T> {
    fn drop(&mut self) {
        self.current.store(self.target, Ordering::Relaxed);
    }
}
