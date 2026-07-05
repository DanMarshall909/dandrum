use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

pub struct CountingAllocator;

thread_local! {
    static IS_TRACKING: Cell<bool> = const { Cell::new(false) };
    static ALLOCATION_COUNT: Cell<usize> = const { Cell::new(0) };
}

#[global_allocator]
static GLOBAL_ALLOCATOR: CountingAllocator = CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        IS_TRACKING.with(|is_tracking| {
            if is_tracking.get() {
                ALLOCATION_COUNT.with(|allocation_count| {
                    allocation_count.set(allocation_count.get() + 1);
                });
            }
        });

        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        IS_TRACKING.with(|is_tracking| {
            if is_tracking.get() {
                ALLOCATION_COUNT.with(|allocation_count| {
                    allocation_count.set(allocation_count.get() + 1);
                });
            }
        });

        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

pub fn count_current_thread_allocations(work: impl FnOnce()) -> usize {
    ALLOCATION_COUNT.with(|allocation_count| allocation_count.set(0));
    IS_TRACKING.with(|is_tracking| is_tracking.set(true));
    work();
    IS_TRACKING.with(|is_tracking| is_tracking.set(false));
    ALLOCATION_COUNT.with(Cell::get)
}
