#![cfg(not(feature = "host"))]
use alloc_cortex_m::CortexMHeap;
use core::alloc::Layout;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicBool, Ordering};
extern crate alloc;

// How much space to reserve for the heap. Most usage for DSTs should use nowhere near 4k of memory,
// but with high power electronics it's better to be safe than sorry.
const HEAP_SIZE_BYTES: usize = 1 << 14;

// Should the allocator ever be unable to allocate due to fragmentation or insufficient space, loop
// here so we know what's wrong. Alternatively, we could panic!, though it's a bit more idiomatic to
// only panic when code itself is wrong, not as the result of run-time issues.
#[alloc_error_handler]
fn oom(_: Layout) -> ! {
    loop {}
}

// Install the global allocator.
#[global_allocator]
static ALLOCATOR: CortexMHeap = CortexMHeap::empty();

// Heaps can be kind of hairy in embedded environments and are particularly prone to memory
// fragmentation due to their small size. However, since dynamically sized types - Trait Objects -
// are fat pointers, any use of them requires allocating space on the heap for the associated fat
// pointer. For this BLDC implementation, we're creating DSTs sparingly and only from the main
// thread.
pub fn initialize_heap() {
    // Simple assurance to make sure heap initialization isn't done more than once.
    static INITIALIZE: AtomicBool = AtomicBool::new(false);
    if INITIALIZE.swap(true, Ordering::Acquire) {
        panic!("Heap already initialized");
    }

    static mut HEAP: [MaybeUninit<u8>; HEAP_SIZE_BYTES] =
        [MaybeUninit::<u8>::uninit(); HEAP_SIZE_BYTES];
    // Safety: Given the check above, we can assume that at the moment we're the only ones holding
    // onto this memory. Pass the memory into the allocator.
    unsafe { ALLOCATOR.init(&mut HEAP) };
}
