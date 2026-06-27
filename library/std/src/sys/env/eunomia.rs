pub use super::common::Env;
use crate::ffi::{OsStr, OsString};
use crate::io;
use crate::sys::FromInner;
use crate::sys::os_str::Buf;

// Provided by the seam crate `eunomia-sys` (see `sys/pal/eunomia/mod.rs`): the stashed
// environment as raw `KEY=VALUE` byte-strings borrowed from the startup block.
unsafe extern "Rust" {
    fn __eunomia_env() -> &'static [&'static [u8]];
}

// Env entries are raw `KEY=VALUE` byte-strings (POSIX `environ` convention, rev2§5.1).
// Split on the first `=`; an entry with no `=` is a key with an empty value.
fn split_kv(entry: &[u8]) -> (&[u8], &[u8]) {
    match entry.iter().position(|&c| c == b'=') {
        Some(i) => (&entry[..i], &entry[i + 1..]),
        None => (entry, &[]),
    }
}

fn to_os(bytes: &[u8]) -> OsString {
    OsString::from_inner(Buf::from_inner(bytes.to_vec()))
}

pub fn env() -> Env {
    Env::new(
        unsafe { __eunomia_env() }
            .iter()
            .map(|entry| {
                let (k, v) = split_kv(entry);
                (to_os(k), to_os(v))
            })
            .collect(),
    )
}

pub fn getenv(key: &OsStr) -> Option<OsString> {
    let want = key.as_encoded_bytes();
    unsafe { __eunomia_env() }.iter().find_map(|entry| {
        let (k, v) = split_kv(entry);
        (k == want).then(|| to_os(v))
    })
}

// No env producer reaches a running process yet (env entries are populated by the
// spawner, std-port 5.2), and there is no shared mutable environ to mutate, so these
// are deliberately unsupported rather than silently no-op.
pub unsafe fn setenv(_: &OsStr, _: &OsStr) -> io::Result<()> {
    Err(io::const_error!(io::ErrorKind::Unsupported, "cannot set env vars on this platform"))
}

pub unsafe fn unsetenv(_: &OsStr) -> io::Result<()> {
    Err(io::const_error!(io::ErrorKind::Unsupported, "cannot unset env vars on this platform"))
}
