use crate::io::ErrorKind;

// Provided by the seam crate `eunomia-sys` (see `sys/pal/eunomia/mod.rs`): the
// proptested io-error classification policy. `classify` returns the `Kind` discriminant
// (`#[repr(u8)]`, the numbering below is its definition order); `message` returns a
// static label. eunomia is a microkernel with no ambient errno (errors are the negative
// `ERR_*` syscall return values) and no signals.
unsafe extern "Rust" {
    fn __eunomia_io_classify(code: i64) -> u8;
    fn __eunomia_io_message(code: i64) -> &'static str;
}

pub fn errno() -> i32 {
    0
}

pub fn is_interrupted(_code: i32) -> bool {
    false
}

pub fn decode_error_kind(code: i32) -> ErrorKind {
    // Mirrors `eunomia_sys::io_error::Kind` (the host-tested single source of truth);
    // its `#[repr(u8)]` discriminants are fixed in lockstep with this match. The 7..
    // discriminants are the std-port 4.3 fs decision table; each is a stable
    // `io::ErrorKind`. `5` (`Kind::Uncategorized`) rides the `_` fallback.
    match unsafe { __eunomia_io_classify(code as i64) } {
        0 => ErrorKind::PermissionDenied,
        1 => ErrorKind::WouldBlock,
        2 => ErrorKind::InvalidInput,
        3 => ErrorKind::OutOfMemory,
        4 => ErrorKind::BrokenPipe,
        6 => ErrorKind::NotFound,
        7 => ErrorKind::NotADirectory,
        8 => ErrorKind::ReadOnlyFilesystem,
        9 => ErrorKind::StaleNetworkFileHandle,
        10 => ErrorKind::InvalidFilename,
        11 => ErrorKind::NotConnected,
        12 => ErrorKind::ResourceBusy,
        _ => ErrorKind::Uncategorized,
    }
}

pub fn error_string(errno: i32) -> String {
    unsafe { __eunomia_io_message(errno as i64) }.to_string()
}
