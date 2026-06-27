use crate::alloc::{GlobalAlloc, Layout, System};

// Provided by the seam crate `eunomia-sys` (see `sys/pal/eunomia/mod.rs` for why
// this is an `extern "Rust"` symbol rather than a direct call): the process-global
// `urt::Heap<N>` over the Verus-verified `freelist` allocator. MVP bounds: the heap
// is single-threaded with no lock (threads are std-port Phase 3); `MAX_ALIGN = 64`
// (a request aligned above 64 — e.g. a page — returns null = clean OOM, not UB); a
// fragmentation cap of 1024 free extents is a second, independent limit (a dealloc
// at the cap leaks the block); and OOM is a hard abort (null -> handle_alloc_error),
// not a graceful `Err`. `N` is a per-binary reservation committed at spawn (no
// demand paging). `realloc`/`alloc_zeroed` use GlobalAlloc's defaults (alloc+copy /
// alloc+memset) — no in-place grow.
unsafe extern "Rust" {
    fn __eunomia_alloc(layout: Layout) -> *mut u8;
    fn __eunomia_dealloc(ptr: *mut u8, layout: Layout);
}

#[stable(feature = "alloc_system_type", since = "1.28.0")]
unsafe impl GlobalAlloc for System {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe { __eunomia_alloc(layout) }
    }

    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { __eunomia_dealloc(ptr, layout) }
    }
}
