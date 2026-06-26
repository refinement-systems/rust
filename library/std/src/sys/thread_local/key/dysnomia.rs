//! Key-based TLS provided through the Dysnomia application ABI.

use crate::sys::pal::{abi, infallible_status};

unsafe extern "C" fn no_tls_destructor(_: *mut u8) {}

/// A TLS key. Zero reports allocation failure and is also `LazyKey`'s
/// uninitialized sentinel.
pub type Key = usize;

#[inline]
pub fn create(dtor: Option<unsafe extern "C" fn(*mut u8)>) -> Key {
    let (dtor, dtor_present) =
        dtor.map_or((no_tls_destructor as abi::TlsDestructor, 0), |dtor| (dtor, 1));
    let mut key = 0;
    let status = unsafe { abi::__dysnomia_pal_v1_tls_create(dtor, dtor_present, &mut key) };
    infallible_status(status);
    if key == 0 {
        rtabort!("out of TLS keys");
    }
    key as usize
}

#[inline]
pub unsafe fn set(key: Key, value: *mut u8) {
    unsafe { abi::__dysnomia_pal_v1_tls_set(key as u64, value) }
}

#[inline]
pub unsafe fn get(key: Key) -> *mut u8 {
    unsafe { abi::__dysnomia_pal_v1_tls_get(key as u64) }
}

#[inline]
pub unsafe fn destroy(key: Key) {
    unsafe { abi::__dysnomia_pal_v1_tls_destroy(key as u64) }
}
