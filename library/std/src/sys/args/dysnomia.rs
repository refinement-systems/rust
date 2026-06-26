pub use super::common::Args;
use crate::ffi::OsString;
use crate::sys::FromInner;
use crate::sys::os_str::Buf;

// Provided by the seam crate `dysnomia-sys` (see `sys/pal/dysnomia/mod.rs` for why this
// is an `extern "Rust"` symbol rather than a direct call): the stashed argv as raw
// byte-strings borrowed from the startup block.
unsafe extern "Rust" {
    fn __dysnomia_argv() -> &'static [&'static [u8]];
}

pub fn args() -> Args {
    // argv arrives as raw byte-strings in the startup block. dysnomia's
    // `OsStr` is the bytes encoding (no WTF-8), so each byte-string maps straight to an
    // `OsString` via the internal `Buf` — no lossy UTF-8 round-trip, no `os::dysnomia`
    // ffi shim.
    Args::new(
        unsafe { __dysnomia_argv() }
            .iter()
            .map(|b| OsString::from_inner(Buf::from_inner(b.to_vec())))
            .collect(),
    )
}
