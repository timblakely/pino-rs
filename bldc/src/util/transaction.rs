//! Transaction/Buffer-Based Primitive
//!
//! A type of synchronization primitive where the reader can preempt the writer. Particularly useful
//! when you've got an interrupt that you cannot interrupt with a critical section e.g. a control
//! loop. This effectively provides eventual consistency on the reads.

use core::sync::atomic::{AtomicIsize, Ordering};

pub struct BufferedTransaction<T: Sized + Copy> {
    buffer: AtomiUIsize,
    value: [T; 2],
}

impl<T: Sized + Copy> BufferedTransaction<T> {
    // TODO(blakely): Ensure this can only be called with a token. See
    // https://www.reddit.com/r/rust/comments/ectw9e/embedded_rust_and_interrupt_handling/fbfbmua?utm_source=share&utm_medium=web2x&context=3
    pub fn read(&self) -> &T {
        match self.buffer.load(Ordering::Acquire) {
            0 => &self.value[0],
            _ => &self.value[1],
        }
    }

    pub fn commit(&mut self, new_value: &T) {
        // Identify which buffer is safe for writing to.
        let target_idx = match self.buffer.load(Ordering::Acquire) {
            0 => 1,
            _ => 0,
        };

        // Write to the currently unused buffer.
        self.value[target_idx] = *new_value;
        // This is the atomic instruction that, when successful, swaps the buffer pointer.
        // Even if this thread is preempted between the above and below instructions, the
        // reader will only get a slightly outdated copy of the protected data.
        self.buffer.store(target_idx, Ordering::Release);
    }
}
