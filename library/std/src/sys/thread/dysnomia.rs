use crate::ffi::CStr;
use crate::io;
use crate::num::NonZeroUsize;
use crate::sys::pal::{abi, status_result};
use crate::thread::ThreadInit;
use crate::time::Duration;

// The minimum stack size requested when the caller does not choose a larger one.
pub const DEFAULT_MIN_STACK_SIZE: usize = 16 * 4096;

pub struct Thread {
    handle: u64,
}

unsafe impl Send for Thread {}
unsafe impl Sync for Thread {}

impl Thread {
    // unsafe: see thread::Builder::spawn_unchecked for safety requirements
    pub unsafe fn new(stack: usize, init: Box<ThreadInit>) -> io::Result<Thread> {
        // Reconstruct the `ThreadInit` box, initialize TLS, run the closure, and
        // terminate through the application ABI.
        unsafe extern "C" fn thread_start(arg: u64) -> ! {
            unsafe {
                abi::__dysnomia_pal_v1_tls_init_thread();
                let init = Box::from_raw(core::ptr::with_exposed_provenance_mut::<ThreadInit>(
                    arg as usize,
                ));
                let rust_start = init.init();
                rust_start();
                // Preserve std's destructor ordering before releasing TLS.
                abi::__dysnomia_pal_v1_tls_run_dtors();
                crate::rt::thread_cleanup();
                abi::__dysnomia_pal_v1_tls_free_thread();
                abi::__dysnomia_pal_v1_thread_exit(0)
            }
        }

        let arg = Box::into_raw(init).expose_provenance() as u64;
        let mut handle = 0;
        let status = unsafe {
            abi::__dysnomia_pal_v1_thread_spawn(thread_start, stack as u64, arg, &mut handle)
        };
        if let Err(error) = status_result(status) {
            // The trampoline never ran, so the boxed `ThreadInit` leaked — reclaim
            // it (the pointer is still valid: spawn failed before enqueue).
            drop(unsafe {
                Box::from_raw(core::ptr::with_exposed_provenance_mut::<ThreadInit>(arg as usize))
            });
            return Err(error);
        }
        Ok(Thread { handle })
    }

    pub fn join(self) {
        let status = unsafe { abi::__dysnomia_pal_v1_thread_join(self.handle) };
        assert!(status == 0, "dysnomia thread join failed ({status})");
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
    // Single-core scheduler.
    Ok(NonZeroUsize::new(1).unwrap())
}

pub fn yield_now() {
    unsafe { abi::__dysnomia_pal_v1_thread_yield() }
}

pub fn sleep(dur: Duration) {
    let nanos = dur.as_nanos().min(u64::MAX as u128) as u64;
    unsafe { abi::__dysnomia_pal_v1_thread_sleep(nanos) }
}
