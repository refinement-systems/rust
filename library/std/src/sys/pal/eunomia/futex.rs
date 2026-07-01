//! Futex over the eunomia seam (std-port 3.3): `sys::futex`, the one primitive that
//! backs the whole upstream lock stack (`Mutex`/`Condvar`/`RwLock`/`Once`/`Parker`).
//!
//! Delegates to the seam crate `eunomia-sys` through `extern "Rust"` symbols (see
//! `sys/pal/eunomia/mod.rs` for why these are undefined externs and not a direct
//! dependency), whose `urt::futex` table emulates the futex over kernel
//! notifications (an address→waiter table + a per-thread park-notif). This arm only
//! marshals the timeout to nanoseconds (the `sys/pal/motor` convention); all logic
//! lives in the seam.

use crate::sync::atomic::Atomic;
use crate::time::Duration;

/// An atomic for use as a futex that is at least 32-bits but may be larger.
pub type Futex = Atomic<Primitive>;
/// Must be the underlying type of Futex.
pub type Primitive = u32;

/// An atomic for use as a futex that is at least 8-bits but may be larger.
pub type SmallFutex = Atomic<SmallPrimitive>;
/// Must be the underlying type of SmallFutex.
pub type SmallPrimitive = u32;

// Provided by the seam crate `eunomia-sys` (the `__rust_alloc` pattern): the
// `urt::futex` notif-backed table. `timeout_ns == u64::MAX` means "no timeout".
unsafe extern "Rust" {
    fn __eunomia_futex_wait(futex: &Atomic<u32>, expected: u32, timeout_ns: u64) -> bool;
    fn __eunomia_futex_wake(futex: &Atomic<u32>) -> bool;
    fn __eunomia_futex_wake_all(futex: &Atomic<u32>);
}

/// Waits for a `futex_wake` operation to wake us, returning directly if the futex
/// does not hold `expected`. Returns false on timeout, true in all other cases.
pub fn futex_wait(futex: &Atomic<u32>, expected: u32, timeout: Option<Duration>) -> bool {
    // Marshal to nanoseconds, with `u64::MAX` reserved as the "no timeout" sentinel
    // (the motor convention). A finite timeout that would saturate onto the sentinel
    // becomes the largest finite timeout instead of a block-forever.
    let timeout_ns = match timeout {
        None => u64::MAX,
        Some(d) => {
            let ns = d.as_nanos().min(u64::MAX as u128) as u64;
            if ns == u64::MAX { u64::MAX - 1 } else { ns }
        }
    };
    unsafe { __eunomia_futex_wait(futex, expected, timeout_ns) }
}

/// Wakes up one thread that is waiting on `futex_wait` on this futex. Returns true
/// if this actually woke up such a thread, false if none was waiting.
pub fn futex_wake(futex: &Atomic<u32>) -> bool {
    unsafe { __eunomia_futex_wake(futex) }
}

/// Wakes up all threads that are waiting on `futex_wait` on this futex.
pub fn futex_wake_all(futex: &Atomic<u32>) {
    unsafe { __eunomia_futex_wake_all(futex) }
}
