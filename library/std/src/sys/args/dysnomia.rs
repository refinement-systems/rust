pub use super::common::Args;
use crate::ffi::OsString;
use crate::sys::FromInner;
use crate::sys::os_str::Buf;
use crate::sys::pal::{abi, borrowed_byte_table, borrowed_bytes, infallible_status};

fn argv() -> &'static [abi::BorrowedBytes] {
    let mut entries = core::ptr::null();
    let mut count = 0;
    let status = unsafe { abi::__dysnomia_pal_v1_argv(&mut entries, &mut count) };
    infallible_status(status);
    unsafe { borrowed_byte_table(entries, count) }
}

pub fn args() -> Args {
    // Dysnomia's `OsStr` encoding is bytes, so no lossy conversion is needed.
    Args::new(
        argv()
            .iter()
            .map(|&b| OsString::from_inner(Buf::from_inner(unsafe { borrowed_bytes(b) }.to_vec())))
            .collect(),
    )
}
