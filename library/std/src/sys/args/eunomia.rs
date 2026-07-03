pub use super::common::Args;
use crate::ffi::OsString;
use crate::sys::FromInner;
use crate::sys::os_str::Buf;

// Provided by the seam crate `eunomia-sys` (see `sys/pal/eunomia/mod.rs` for why this
// is an `extern "Rust"` symbol rather than a direct call): the stashed argv as raw
// byte-strings borrowed from the startup block.
unsafe extern "Rust" {
    fn __eunomia_argv() -> &'static [&'static [u8]];
}

pub fn args() -> Args {
    // argv arrives as raw byte-strings in the startup block (rev3§5.1). eunomia's
    // `OsStr` is the bytes encoding (no WTF-8), so each byte-string maps straight to an
    // `OsString` via the internal `Buf` — no lossy UTF-8 round-trip, no `os::eunomia`
    // ffi shim.
    Args::new(
        unsafe { __eunomia_argv() }
            .iter()
            .map(|b| OsString::from_inner(Buf::from_inner(b.to_vec())))
            .collect(),
    )
}
