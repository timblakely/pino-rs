use alloc_cortex_m::CortexMHeap;
use core::alloc::Layout;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicBool, Ordering};
extern crate alloc;

#[alloc_error_handler]
fn oom(_: Layout) -> ! {
    loop {}
}

#[global_allocator]
static ALLOCATOR: CortexMHeap = CortexMHeap::empty();

pub fn initialize_heap() {
    // Simple assurance to make sure heap initialization isn't done more than once.
    static INITIALIZE: AtomicBool = AtomicBool::new(false);
    if INITIALIZE.swap(true, Ordering::Acquire) {
        panic!("Heap already initialized");
    }

    const HEAP_SIZE_BYTES: usize = 4096;
    static mut HEAP: [MaybeUninit<u8>; HEAP_SIZE_BYTES] =
        [MaybeUninit::<u8>::uninit(); HEAP_SIZE_BYTES];
    // Safety: Given the check above, we can assume that at the moment we're the only ones holding
    // onto this memory. Pass the memory into the allocator.
    unsafe { ALLOCATOR.init(&mut HEAP) };
}
