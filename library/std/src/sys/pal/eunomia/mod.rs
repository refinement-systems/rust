#![deny(unsafe_op_in_unsafe_fn)]

mod common;
pub use common::*;

// The `sys::futex` backend (std-port 3.3): surfaced as `crate::sys::futex` by the
// `sys/mod.rs` `pub use pal::*` glob (mirroring `sys/pal/motor`'s `pub use
// moto_rt::futex`), so the five `sys/sync/*` dispatchers pick the futex impls.
pub mod futex;

// The eunomia PAL↔seam ABI (the `__rust_alloc` pattern). The seam crate `eunomia-sys`
// cannot be a std/sysroot dependency — its verified deps pull `vstd`, whose
// `verus_builtin` is not buildable as a `rustc-dep-of-std` sysroot crate. So std
// declares the small set of symbols it needs as undefined `extern "Rust"`, and a std
// binary links `eunomia-sys` (an ordinary dep) which `#[no_mangle]`-defines them; they
// resolve at final link. The same handful of symbols is re-declared, narrowly, in each
// consuming arm (`sys/args/eunomia.rs`, `sys/env/eunomia.rs`, `sys/io/error/eunomia.rs`)
// — they all name the one external symbol. Everything across this seam is verified or
// host-tested inside `eunomia-sys`; the PAL only marshals.
unsafe extern "Rust" {
    /// Point the main thread's `TPIDR_EL0` at its TLS block (std-port 3.2), before any
    /// `local_pointer!` access (`set_current` on the main thread).
    fn __eunomia_tls_init_main();
    /// Receive + verified-decode the slot-0 startup block and stash argv/env/grants.
    fn __eunomia_bootstrap_init();
    /// Exit through the kernel thread-exit terminus (rev2§5.1); the parent reaper reads
    /// `code` as the child's status.
    fn __eunomia_thread_exit(code: u64) -> !;
}

// The non-crt0 process entry (rev2§5.1). Eunomia has no C runtime: the ELF entry is
// `_start` (the `ENTRY(_start)` link.ld convention, rust-lld's default entry symbol).
#[cfg(not(test))]
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    // Real per-thread TLS for the main thread (std-port 3.2): point `TPIDR_EL0` at
    // its block before anything (bootstrap or the `main`/`lang_start` rt) touches a
    // `local_pointer!` (the current-thread handle/id).
    unsafe { __eunomia_tls_init_main() };

    // Receive + verified-decode the bootstrap block and stash it (so `sys::args`,
    // `sys::env`, and later grant lookups have data) before `main` runs.
    unsafe { __eunomia_bootstrap_init() };

    // The compiler generates `main` (the `lang_start` wrapper) with the C signature
    // `(argc, argv, sigpipe) -> i32`. argv/env do not arrive that way on eunomia (they
    // ride the startup block, already stashed above), so pass the empty `motor` shape.
    unsafe extern "C" {
        fn main(argc: isize, argv: *const *const u8, sigpipe: u8) -> i32;
    }
    let code = unsafe { main(0, core::ptr::null(), 0) };

    // Orderly exit: the parent reaper reads this status. A panic in a std binary instead
    // routes through `abort_internal` → `thread_exit(STATUS_PANIC)` (overridden in
    // std-port 2.3), which the reaper distinguishes from `exit(0)`. Zero-extend the
    // `i32` (`code as u32 as u64`, in lockstep with the `sys::exit::exit` eunomia arm):
    // sign-extending `-1` would land on `u64::MAX == STATUS_PANIC` and reap a clean
    // `main`-returns-`-1` as a crash.
    unsafe { __eunomia_thread_exit(code as u32 as u64) }
}
