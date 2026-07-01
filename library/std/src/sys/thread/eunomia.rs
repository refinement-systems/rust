use crate::ffi::CStr;
use crate::io;
use crate::num::NonZeroUsize;
use crate::thread::ThreadInit;
use crate::time::Duration;

// Provided by the seam crate `eunomia-sys` (see `sys/pal/eunomia/mod.rs` for why
// these are `extern "Rust"` symbols rather than direct calls): the in-process
// thread primitive over `urt::thread` (std-port 3.2). `spawn` shares the process's
// address space (via `thread_start_as`, op 18) and passes the closure pointer in
// the new thread's initial `x0` (the op-18 seventh arg register), so the trampoline
// below is a plain `extern "C" fn(u64)` — the `sys/thread/motor.rs` shape, minus the
// naked stub. `spawn` returns a join handle (>= 0) or a negative `ERR_*`, surfaced
// through `from_raw_os_error`; an unconfigured (non-thread-capable) process gets
// `ERR_STATE`. All thread logic lives in the seam; this arm only marshals.
unsafe extern "Rust" {
    fn __eunomia_thread_spawn(entry: usize, stack: usize, arg: u64) -> i64;
    fn __eunomia_thread_join(handle: u64) -> i64;
    fn __eunomia_thread_yield();
    fn __eunomia_thread_sleep(nanos: u64);
    fn __eunomia_thread_exit(code: u64) -> !;
    /// Set up this spawned thread's `TPIDR_EL0` TLS block (std-port 3.2) before
    /// `ThreadInit::init` runs `set_current` (which needs per-thread storage).
    fn __eunomia_tls_init_thread();
}

// The fixed thread-stack size the seam maps (`urt::thread_layout::STACK_PAGES *
// PAGE` = 16 * 4096 = 64 KiB). A `Builder::stack_size` above this is refused by the
// seam (`ERR_ARG`) — an MVP bound.
pub const DEFAULT_MIN_STACK_SIZE: usize = 16 * 4096;

pub struct Thread {
    handle: u64,
}

unsafe impl Send for Thread {}
unsafe impl Sync for Thread {}

impl Thread {
    // unsafe: see thread::Builder::spawn_unchecked for safety requirements
    pub unsafe fn new(stack: usize, init: Box<ThreadInit>) -> io::Result<Thread> {
        // The child entry (runs on the new thread's own stack): reconstruct the
        // `ThreadInit` box from the arg the kernel placed in `x0`, set up the
        // current-thread handle + run the closure (`ThreadInit::init` allocates on
        // the shared heap — serialized by the heap spinlock, std-port 3.2), then
        // exit through the kernel terminus so the joiner's notification fires.
        extern "C" fn thread_start(arg: u64) -> ! {
            unsafe {
                // Real per-thread TLS first (std-port 3.2): `TPIDR_EL0` must point at
                // this thread's block before `init.init()` runs `set_current`.
                __eunomia_tls_init_thread();
                let init = Box::from_raw(core::ptr::with_exposed_provenance_mut::<ThreadInit>(
                    arg as usize,
                ));
                let rust_start = init.init();
                rust_start();
                __eunomia_thread_exit(0)
            }
        }

        let arg = Box::into_raw(init).expose_provenance() as u64;
        let h = unsafe { __eunomia_thread_spawn(thread_start as usize, stack, arg) };
        if h < 0 {
            // The trampoline never ran, so the boxed `ThreadInit` leaked — reclaim
            // it (the pointer is still valid: spawn failed before enqueue).
            drop(unsafe {
                Box::from_raw(core::ptr::with_exposed_provenance_mut::<ThreadInit>(arg as usize))
            });
            return Err(io::Error::from_raw_os_error(h as i32));
        }
        Ok(Thread { handle: h as u64 })
    }

    pub fn join(self) {
        let r = unsafe { __eunomia_thread_join(self.handle) };
        assert!(r >= 0, "eunomia thread join failed ({r})");
    }
}

pub fn set_name(_name: &CStr) {
    // No per-thread OS name (the SGX arm's posture); the std-visible thread name is
    // already kept by the platform-agnostic Rust thread code.
}

pub fn current_os_id() -> Option<u64> {
    None
}

pub fn available_parallelism() -> io::Result<NonZeroUsize> {
    // Single-core scheduler (rev2§5.4).
    Ok(NonZeroUsize::new(1).unwrap())
}

pub fn yield_now() {
    unsafe { __eunomia_thread_yield() }
}

pub fn sleep(dur: Duration) {
    let nanos = dur.as_nanos().min(u64::MAX as u128) as u64;
    unsafe { __eunomia_thread_sleep(nanos) }
}
