use alloc_cortex_m::CortexMHeap;
use core::alloc::Layout;
use core::mem::MaybeUninit;

#[alloc_error_handler]
fn oom(_: Layout) -> ! {
    loop {}
}

#[global_allocator]
static ALLOCATOR: CortexMHeap = CortexMHeap::empty();

pub fn initialize_heap(heap_loc: &'static mut [MaybeUninit<u8>]) {
    unsafe { ALLOCATOR.init(heap_loc) };
}
