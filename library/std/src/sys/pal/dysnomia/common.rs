use super::abi::{self, BorrowedBytes};
use crate::io;

// SAFETY: must be called only once during runtime initialization.
// NOTE: this is not guaranteed to run, for example when Rust code is called externally.
pub unsafe fn init(_argc: isize, _argv: *const *const u8, _sigpipe: u8) {}

// SAFETY: must be called only once during runtime cleanup.
// NOTE: this is not guaranteed to run, for example when the program aborts.
pub unsafe fn cleanup() {}

pub fn unsupported<T>() -> io::Result<T> {
    Err(unsupported_err())
}

pub fn unsupported_err() -> io::Error {
    io::Error::UNSUPPORTED_PLATFORM
}

pub(crate) fn status_result(status: i32) -> io::Result<()> {
    match status {
        0 => Ok(()),
        ..=-1 => Err(io::Error::from_raw_os_error(status)),
        _ => Err(io::const_error!(
            io::ErrorKind::InvalidData,
            "Dysnomia PAL returned an invalid positive status"
        )),
    }
}

pub(crate) fn invalid_data(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

pub(crate) fn count_result(status: i32, count: u64, limit: usize) -> io::Result<usize> {
    status_result(status)?;
    let count = usize::try_from(count).map_err(|_| {
        io::const_error!(io::ErrorKind::InvalidData, "Dysnomia PAL count does not fit usize")
    })?;
    if count > limit {
        return Err(io::const_error!(
            io::ErrorKind::InvalidData,
            "Dysnomia PAL count exceeds the supplied buffer"
        ));
    }
    Ok(count)
}

pub(crate) fn abi_bool(value: u32) -> io::Result<bool> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(io::const_error!(
            io::ErrorKind::InvalidData,
            "Dysnomia PAL returned an invalid boolean"
        )),
    }
}

pub(crate) fn infallible_abi_bool(value: u32) -> bool {
    match value {
        0 => false,
        1 => true,
        _ => abort_internal(),
    }
}

pub(crate) unsafe fn borrowed_bytes(bytes: BorrowedBytes) -> &'static [u8] {
    let Ok(len) = usize::try_from(bytes.len) else { abort_internal() };
    if len == 0 {
        return &[];
    }
    if bytes.ptr.is_null() {
        abort_internal()
    }
    unsafe { core::slice::from_raw_parts(bytes.ptr, len) }
}

pub(crate) unsafe fn borrowed_byte_table(
    entries: *const BorrowedBytes,
    count: u64,
) -> &'static [BorrowedBytes] {
    let Ok(count) = usize::try_from(count) else { abort_internal() };
    if count == 0 {
        return &[];
    }
    if entries.is_null()
        || !(entries as usize).is_multiple_of(core::mem::align_of::<BorrowedBytes>())
    {
        abort_internal()
    }
    unsafe { core::slice::from_raw_parts(entries, count) }
}

pub(crate) fn infallible_status(status: i32) {
    if status != 0 {
        abort_internal()
    }
}

pub fn abort_internal() -> ! {
    // The application-provided terminator reserves `u64::MAX` for abnormal exit.
    unsafe { abi::__dysnomia_pal_v1_thread_exit(u64::MAX) }
}
