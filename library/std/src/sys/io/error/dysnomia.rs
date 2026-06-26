use crate::io::ErrorKind;
use crate::sys::pal::{abi, borrowed_bytes};

pub fn errno() -> i32 {
    0
}

pub fn is_interrupted(_code: i32) -> bool {
    false
}

pub fn decode_error_kind(code: i32) -> ErrorKind {
    // These discriminants are part of the PAL ABI v1 classification contract.
    // Unrecognized values map to `Uncategorized`.
    match unsafe { abi::__dysnomia_pal_v1_io_classify(code) } {
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
    let message = unsafe { abi::__dysnomia_pal_v1_io_message(errno) };
    String::from_utf8_lossy(unsafe { borrowed_bytes(message) }).into_owned()
}
