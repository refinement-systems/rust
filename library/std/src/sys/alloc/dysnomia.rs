use crate::alloc::{GlobalAlloc, Layout, System};
use crate::sys::pal::abi;

#[stable(feature = "alloc_system_type", since = "1.28.0")]
unsafe impl GlobalAlloc for System {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe { abi::__dysnomia_pal_v1_alloc(layout.size() as u64, layout.align() as u64) }
    }

    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { abi::__dysnomia_pal_v1_dealloc(ptr, layout.size() as u64, layout.align() as u64) }
    }
}
