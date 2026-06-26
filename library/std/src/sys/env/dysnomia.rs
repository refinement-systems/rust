pub use super::common::Env;
use crate::ffi::{OsStr, OsString};
use crate::io;
use crate::sys::FromInner;
use crate::sys::os_str::Buf;
use crate::sys::pal::{abi, borrowed_byte_table, borrowed_bytes, infallible_status};

fn entries() -> &'static [abi::BorrowedBytes] {
    let mut entries = core::ptr::null();
    let mut count = 0;
    let status = unsafe { abi::__dysnomia_pal_v1_env(&mut entries, &mut count) };
    infallible_status(status);
    unsafe { borrowed_byte_table(entries, count) }
}

// Env entries are raw `KEY=VALUE` byte-strings (POSIX `environ` convention).
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
        entries()
            .iter()
            .map(|&entry| {
                let entry = unsafe { borrowed_bytes(entry) };
                let (k, v) = split_kv(entry);
                (to_os(k), to_os(v))
            })
            .collect(),
    )
}

pub fn getenv(key: &OsStr) -> Option<OsString> {
    let want = key.as_encoded_bytes();
    entries().iter().find_map(|&entry| {
        let entry = unsafe { borrowed_bytes(entry) };
        let (k, v) = split_kv(entry);
        (k == want).then(|| to_os(v))
    })
}

// The Dysnomia ABI exposes a read-only environment.
pub unsafe fn setenv(_: &OsStr, _: &OsStr) -> io::Result<()> {
    Err(io::const_error!(io::ErrorKind::Unsupported, "cannot set env vars on this platform"))
}

pub unsafe fn unsetenv(_: &OsStr) -> io::Result<()> {
    Err(io::const_error!(io::ErrorKind::Unsupported, "cannot unset env vars on this platform"))
}
