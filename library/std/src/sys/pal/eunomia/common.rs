use crate::io as std_io;

// SAFETY: must be called only once during runtime initialization.
// NOTE: this is not guaranteed to run, for example when Rust code is called externally.
pub unsafe fn init(_argc: isize, _argv: *const *const u8, _sigpipe: u8) {}

// SAFETY: must be called only once during runtime cleanup.
// NOTE: this is not guaranteed to run, for example when the program aborts.
pub unsafe fn cleanup() {}

pub fn unsupported<T>() -> std_io::Result<T> {
    Err(unsupported_err())
}

pub fn unsupported_err() -> std_io::Error {
    std_io::Error::UNSUPPORTED_PLATFORM
}

pub fn abort_internal() -> ! {
    // std owns the panic handler in a std binary, so the rev3§5.1 reaper contract is
    // preserved here: panic, OOM, and `process::abort()` all funnel through
    // `crate::sys::abort_internal` (panic -> `__rust_abort` -> `process::abort` -> here),
    // so this single override makes every abnormal stop reap as the reserved panic
    // status, distinct from any `exit(code)`. The all-ones literal is
    // `eunomia_sys::syscall::STATUS_PANIC` (`u64::MAX`), duplicated here because std
    // cannot depend on the seam crate — the same posture as the `ERR_*` discriminants in
    // `sys/io/error/eunomia.rs`; it stays in lockstep with that crate.
    unsafe extern "Rust" {
        fn __eunomia_thread_exit(code: u64) -> !;
    }
    unsafe { __eunomia_thread_exit(u64::MAX) }
}
