//! Separate test-only Rust cdylib using the same default System allocator route.
use std::alloc::{alloc,dealloc,Layout};
#[unsafe(no_mangle)]
pub unsafe extern "C" fn probe_allocate() -> *mut u8 {
    let pointer=unsafe {alloc(Layout::from_size_align(64,8).unwrap())};
    if !pointer.is_null() {unsafe {pointer.write_volatile(42)};}
    pointer
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn probe_deallocate(pointer:*mut u8) {
    unsafe {dealloc(pointer,Layout::from_size_align(64,8).unwrap())};
}
