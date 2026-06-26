#![deny(unsafe_op_in_unsafe_fn)]

mod common;
pub use common::*;

// The `sys::futex` backend: surfaced as `crate::sys::futex` by the
// `sys/mod.rs` `pub use pal::*` glob (mirroring `sys/pal/motor`'s `pub use
// moto_rt::futex`), so the five `sys/sync/*` dispatchers pick the futex impls.
pub mod futex;

// The dysnomia PAL↔seam ABI (the `__rust_alloc` pattern). The seam crate `dysnomia-sys`
// cannot be a std/sysroot dependency — its verified deps pull `vstd`, whose
// `verus_builtin` is not buildable as a `rustc-dep-of-std` sysroot crate. So std
// declares the small set of symbols it needs as undefined `extern "Rust"`, and a std
// binary links `dysnomia-sys` (an ordinary dep) which `#[no_mangle]`-defines them; they
// resolve at final link. The same handful of symbols is re-declared, narrowly, in each
// consuming arm (`sys/args/dysnomia.rs`, `sys/env/dysnomia.rs`, `sys/io/error/dysnomia.rs`)
// — they all name the one external symbol. Everything across this seam is verified or
// host-tested inside `dysnomia-sys`; the PAL only marshals.
unsafe extern "Rust" {
    /// Point the main thread's `TPIDR_EL0` at its TLS block, before any
    /// `local_pointer!` access (`set_current` on the main thread).
    fn __dysnomia_tls_init_main();
    /// Receive + verified-decode the slot-0 startup block and stash argv/env/grants.
    fn __dysnomia_bootstrap_init();
    /// Exit through the kernel thread-exit terminus; the parent reaper reads
    /// `code` as the child's status.
    fn __dysnomia_thread_exit(code: u64) -> !;
    /// Run the main thread's `thread_local!` destructors at exit.
    fn __dysnomia_tls_run_dtors();
}

// The non-crt0 process entry. Dysnomia has no C runtime: the ELF entry is
// `_start` (the `ENTRY(_start)` link.ld convention, rust-lld's default entry symbol).
#[cfg(not(test))]
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    // Real per-thread TLS for the main thread: point `TPIDR_EL0` at
    // its block before anything (bootstrap or the `main`/`lang_start` rt) touches a
    // `local_pointer!` (the current-thread handle/id).
    unsafe { __dysnomia_tls_init_main() };

    // Receive + verified-decode the bootstrap block and stash it (so `sys::args`,
    // `sys::env`, and later grant lookups have data) before `main` runs.
    unsafe { __dysnomia_bootstrap_init() };

    // The compiler generates `main` (the `lang_start` wrapper) with the C signature
    // `(argc, argv, sigpipe) -> i32`. argv/env do not arrive that way on dysnomia (they
    // ride the startup block, already stashed above), so pass the empty `motor` shape.
    unsafe extern "C" {
        fn main(argc: isize, argv: *const *const u8, sigpipe: u8) -> i32;
    }
    let code = unsafe { main(0, core::ptr::null(), 0) };

    // Main-thread TLS teardown: run the main thread's `thread_local!`
    // destructors and drop its current-thread handle. `lang_start_internal` never
    // calls `thread_cleanup` for main, so this is additive, not a double free. The
    // block is the static `.bss` `MAIN`, so there is nothing to reclaim.
    unsafe {
        __dysnomia_tls_run_dtors();
        crate::rt::thread_cleanup();
    }

    // Orderly exit: the parent reaper reads this status. A panic in a std binary instead
    // routes through `abort_internal` → `thread_exit(STATUS_PANIC)`
    // which the reaper distinguishes from `exit(0)`. Zero-extend the
    // `i32` (`code as u32 as u64`, in lockstep with the `sys::exit::exit` dysnomia arm):
    // sign-extending `-1` would land on `u64::MAX == STATUS_PANIC` and reap a clean
    // `main`-returns-`-1` as a crash.
    unsafe { __dysnomia_thread_exit(code as u32 as u64) }
}
