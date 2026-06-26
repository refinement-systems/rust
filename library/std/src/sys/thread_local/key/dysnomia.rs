//! Dysnomia's key-based TLS backend: TLS keys over the seam crate
//! `dysnomia_sys::tls` — the verified `urt::tls` key table plus the `TPIDR_EL0`
//! per-thread pointer block — reached through `__dysnomia_tls_*` `extern "Rust"`
//! symbols like every other dysnomia PAL surface (the seam crate cannot be a sysroot
//! dependency; see `sys/pal/dysnomia/mod.rs`). `os.rs` storage drives these five
//! symbols; `racy::LazyKey` creates keys lazily. The `sys/thread/motor.rs` shape,
//! minus the direct `moto_rt` dependency.

// Provided by the seam crate `dysnomia-sys`: the key allocator + per-thread value
// cells + the thread-exit destructor runner.
unsafe extern "Rust" {
    fn __dysnomia_tls_create(dtor: Option<unsafe extern "C" fn(*mut u8)>) -> usize;
    fn __dysnomia_tls_get(key: usize) -> *mut u8;
    fn __dysnomia_tls_set(key: usize, val: *mut u8);
    fn __dysnomia_tls_destroy(key: usize);
}

/// A TLS key: a 1-based index into this thread's `TPIDR` block. `0` is the seam's
/// "table full" return and `racy::LazyKey`'s uninitialized sentinel, so a real key
/// is never `0` — `create` aborts rather than hand one out.
pub type Key = usize;

#[inline]
pub fn create(dtor: Option<unsafe extern "C" fn(*mut u8)>) -> Key {
    let key = unsafe { __dysnomia_tls_create(dtor) };
    if key == 0 {
        rtabort!("out of TLS keys");
    }
    key
}

#[inline]
pub unsafe fn set(key: Key, value: *mut u8) {
    unsafe { __dysnomia_tls_set(key, value) }
}

#[inline]
pub unsafe fn get(key: Key) -> *mut u8 {
    unsafe { __dysnomia_tls_get(key) }
}

#[inline]
pub unsafe fn destroy(key: Key) {
    unsafe { __dysnomia_tls_destroy(key) }
}
