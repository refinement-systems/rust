//! Per-thread thread-local storage for eunomia (std-port 3.2): `local_pointer!`
//! over a `TPIDR_EL0`-based per-thread block.
//!
//! Eunomia has real per-thread TLS — 3.1 makes `TPIDR_EL0` survive a context
//! switch, and the seam (`eunomia_sys::tls`) points it at a per-thread block of
//! pointer slots (a zeroed `[*mut (); N]`): the main thread's in `_start`, each
//! spawned thread's in the `sys/thread` trampoline before `ThreadInit::init`. Each
//! `local_pointer!` site claims one slot index at first access — the same index in
//! every thread's block, exactly as a `#[thread_local]` variable has one offset —
//! so `get`/`set` read `[TPIDR + slot]`, i.e. per-thread storage. This is what
//! `set_current` needs to run on more than one thread.
//!
//! The `thread_local!` macro storage (`EagerStorage`/`LazyStorage`) stays the
//! single-threaded `no_threads` version for now (no user `thread_local!` runs in
//! the multi-threaded path yet — `HashMap`'s is std-port 3.4); the verified `urt`
//! key table and destructors are std-port 3.5. Only `local_pointer!` (the
//! current-thread handle and id) needs per-thread storage at 3.2.

use crate::sync::atomic::{AtomicUsize, Ordering};

/// Pointer slots per thread. **MUST match `eunomia_sys::tls::TLS_SLOTS`** — the
/// seam allocates a block of exactly this many, and an out-of-range slot would read
/// past it. Comfortably above std's handful of `local_pointer!` sites (plus a
/// little slack for the rare race where two threads first-touch the same site
/// concurrently and one claim is wasted).
const TLS_SLOTS: usize = 64;

/// Process-global slot allocator. A `local_pointer!` site claims the next free
/// index on first access; every thread then uses that index in its own block.
static NEXT_SLOT: AtomicUsize = AtomicUsize::new(0);

#[rustc_macro_transparency = "semiopaque"]
pub(crate) macro local_pointer {
    () => {},
    ($vis:vis static $name:ident; $($rest:tt)*) => {
        $vis static $name: $crate::sys::thread_local::LocalPointer = $crate::sys::thread_local::LocalPointer::__new();
        $crate::sys::thread_local::local_pointer! { $($rest)* }
    },
}

pub(crate) struct LocalPointer {
    /// `0` = unassigned; otherwise the claimed slot index `+ 1`.
    slot: AtomicUsize,
}

impl LocalPointer {
    pub const fn __new() -> LocalPointer {
        LocalPointer { slot: AtomicUsize::new(0) }
    }

    #[inline]
    fn index(&self) -> usize {
        let s = self.slot.load(Ordering::Acquire);
        if s != 0 { s - 1 } else { self.assign() }
    }

    #[cold]
    fn assign(&self) -> usize {
        let new = NEXT_SLOT.fetch_add(1, Ordering::Relaxed);
        assert!(new < TLS_SLOTS, "eunomia TLS: too many local_pointer sites");
        // Publish our claim; if another thread claimed this same site first, use
        // theirs (ours just leaves an unused slot — safe, bounded by the slack).
        match self.slot.compare_exchange(0, new + 1, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => new,
            Err(existing) => existing - 1,
        }
    }

    pub fn get(&self) -> *mut () {
        // SAFETY: `index() < TLS_SLOTS`, and the seam points TPIDR at a live
        // `[*mut (); TLS_SLOTS]` before any access on this thread.
        unsafe { *tpidr_block().add(self.index()) }
    }

    pub fn set(&self, p: *mut ()) {
        // SAFETY: as in `get`.
        unsafe { *tpidr_block().add(self.index()) = p }
    }
}

// SAFETY: each thread only ever touches its own TPIDR block (per-thread by
// construction); the `slot` atomic is the sole shared state and is synchronized.
unsafe impl Sync for LocalPointer {}

/// This thread's TLS block base — the `TPIDR_EL0` the seam set up (rev2§6.1(d); 3.1
/// preserves it across context switches).
#[inline]
fn tpidr_block() -> *mut *mut () {
    let v: usize;
    // SAFETY: `mrs tpidr_el0` is unconditionally readable at EL0; the seam
    // guarantees it points at a live block before any `local_pointer!` access.
    unsafe {
        core::arch::asm!("mrs {v}, tpidr_el0", v = out(reg) v, options(nomem, nostack, preserves_flags));
    }
    v as *mut *mut ()
}
