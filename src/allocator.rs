extern crate alloc;

use buddy_alloc::{BuddyAllocParam, FastAllocParam, NonThreadsafeAlloc};
use core::alloc::{Layout, GlobalAlloc};
use cortex_m::asm;

// 16k fast Heap
const FAST_HEAP_SIZE: usize = 16 * 1024;

// 32k slow heap
const HEAP_SIZE: usize = 32 * 1024;

// 16 bytes leaf
const LEAF_SIZE: usize = 16;

pub static mut FAST_HEAP: [u8; FAST_HEAP_SIZE] = [0u8; FAST_HEAP_SIZE];
pub static mut HEAP: [u8; HEAP_SIZE] = [0u8; HEAP_SIZE];

#[cfg_attr(not(test), global_allocator)]
static ALLOC: CortexMSafeAlloc = unsafe {
    let fast_param = FastAllocParam::new(FAST_HEAP.as_ptr(), FAST_HEAP_SIZE);
    let buddy_param = BuddyAllocParam::new(HEAP.as_ptr(), HEAP_SIZE, LEAF_SIZE);
    CortexMSafeAlloc(NonThreadsafeAlloc::new(fast_param, buddy_param))
};

#[alloc_error_handler]
fn alloc_error(layout: Layout) -> ! {
    error!("Failed to allocate {}", layout);
    asm::bkpt();
    loop {}
}

pub struct CortexMSafeAlloc(
    pub NonThreadsafeAlloc,
);

unsafe impl GlobalAlloc for CortexMSafeAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        cortex_m::interrupt::free(|_cs| self.0.alloc(layout))
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        cortex_m::interrupt::free(|_cs| self.0.dealloc(ptr, layout))
    }
}



