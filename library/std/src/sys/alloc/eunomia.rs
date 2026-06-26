use crate::alloc::{GlobalAlloc, Layout, System};
use crate::ptr;

// Bring-up stub: std links all-unsupported for `os = "eunomia"`. There is no
// global heap yet, so every allocation fails (returns null) and routes through
// `handle_alloc_error`. A real allocator backing replaces this in a later phase.
#[stable(feature = "alloc_system_type", since = "1.28.0")]
unsafe impl GlobalAlloc for System {
    #[inline]
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
        ptr::null_mut()
    }

    #[inline]
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}
