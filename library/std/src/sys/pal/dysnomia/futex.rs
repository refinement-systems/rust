//! Futex operations provided through the Dysnomia application ABI.

use crate::sync::atomic::Atomic;
use crate::sys::pal::{abi, infallible_abi_bool};
use crate::time::Duration;

/// An atomic for use as a futex that is at least 32-bits but may be larger.
pub type Futex = Atomic<Primitive>;
/// Must be the underlying type of Futex.
pub type Primitive = u32;

/// An atomic for use as a futex that is at least 8-bits but may be larger.
pub type SmallFutex = Atomic<SmallPrimitive>;
/// Must be the underlying type of SmallFutex.
pub type SmallPrimitive = u32;

/// Waits for a `futex_wake` operation to wake us, returning directly if the futex
/// does not hold `expected`. Returns false on timeout, true in all other cases.
pub fn futex_wait(futex: &Atomic<u32>, expected: u32, timeout: Option<Duration>) -> bool {
    // A finite timeout that would saturate to the sentinel becomes the largest
    // finite timeout.
    let timeout_ns = match timeout {
        None => u64::MAX,
        Some(d) => {
            let ns = d.as_nanos().min(u64::MAX as u128) as u64;
            if ns == u64::MAX { u64::MAX - 1 } else { ns }
        }
    };
    infallible_abi_bool(unsafe {
        abi::__dysnomia_pal_v1_futex_wait(futex.as_ptr(), expected, timeout_ns)
    })
}

/// Wakes up one thread that is waiting on `futex_wait` on this futex. Returns true
/// if this actually woke up such a thread, false if none was waiting.
pub fn futex_wake(futex: &Atomic<u32>) -> bool {
    infallible_abi_bool(unsafe { abi::__dysnomia_pal_v1_futex_wake(futex.as_ptr()) })
}

/// Wakes up all threads that are waiting on `futex_wait` on this futex.
pub fn futex_wake_all(futex: &Atomic<u32>) {
    unsafe { abi::__dysnomia_pal_v1_futex_wake_all(futex.as_ptr()) }
}
