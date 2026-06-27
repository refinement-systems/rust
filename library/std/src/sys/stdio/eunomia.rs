use crate::io::{self, BorrowedCursor, IoSliceMut};

// Provided by the seam crate `eunomia-sys` (see `sys/pal/eunomia/mod.rs`): write `buf`
// to the kernel debug-log (rev2§7's EL0 debug-print scaffold), split into the
// ≤1024-byte chunks the kernel accepts, and report the full length written (the path is
// infallible/best-effort — a silent no-op when the kernel lacks the `debug-log`
// feature). All chunking/marshalling lives in the seam; this arm only delegates.
//
// Routing stdout/stderr to this ambient kernel-diagnostic path is a disclosed,
// *temporary* deviation from the rev2§2 capability model (rev2§2.7): phase 5.1 (std-port)
// moves stdout/stdin onto the userspace console channel, and only panic last-words stay
// here (rev2§7 C-M9, "kept ... for kernel-internal panic reporting").
unsafe extern "Rust" {
    fn __eunomia_stdio_write(buf: &[u8]) -> usize;
}

fn debug_write(buf: &[u8]) -> io::Result<usize> {
    // SAFETY: the seam fn is a pure delegation — it reads `buf` (a valid slice) and
    // issues the `DebugWrite` syscall; it allocates nothing and returns the byte count.
    Ok(unsafe { __eunomia_stdio_write(buf) })
}

pub struct Stdin;
pub struct Stdout;
pub struct Stderr;

impl Stdin {
    pub const fn new() -> Stdin {
        Stdin
    }
}

// Stdin is deliberately unassigned until the userspace console lands (rev2§7); reads
// report EOF. This is the `unsupported` `io::Read` surface verbatim; phase 5.1 replaces
// it with the console channel.
impl io::Read for Stdin {
    #[inline]
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        Ok(0)
    }

    #[inline]
    fn read_buf(&mut self, _cursor: BorrowedCursor<'_, u8>) -> io::Result<()> {
        Ok(())
    }

    #[inline]
    fn read_vectored(&mut self, _bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        Ok(0)
    }

    #[inline]
    fn is_read_vectored(&self) -> bool {
        false
    }

    #[inline]
    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        if !buf.is_empty() { Err(io::Error::READ_EXACT_EOF) } else { Ok(()) }
    }

    #[inline]
    fn read_buf_exact(&mut self, cursor: BorrowedCursor<'_, u8>) -> io::Result<()> {
        if cursor.capacity() != 0 { Err(io::Error::READ_EXACT_EOF) } else { Ok(()) }
    }

    #[inline]
    fn read_to_end(&mut self, _buf: &mut Vec<u8>) -> io::Result<usize> {
        Ok(0)
    }

    #[inline]
    fn read_to_string(&mut self, _buf: &mut String) -> io::Result<usize> {
        Ok(0)
    }
}

impl Stdout {
    pub const fn new() -> Stdout {
        Stdout
    }
}

impl io::Write for Stdout {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        debug_write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Stderr {
    pub const fn new() -> Stderr {
        Stderr
    }
}

impl io::Write for Stderr {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        debug_write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

// Separate `Stdin`/`Stdout`/`Stderr` (the `motor` shape, not the `unsupported` alias) so
// phase 5.1 re-points only the `Stdout`/`Stderr` bodies (console / `NAME_STDERR`) without
// reshaping the types.

pub const STDIN_BUF_SIZE: usize = 0;

pub fn is_ebadf(_err: &io::Error) -> bool {
    // `debug_write` never fails, so this is vacuous; `true` is the strictly-safe answer
    // (treat any surfaced error as a missing stream the writer may swallow).
    true
}

// A dedicated writer for panic last-words on the debug-log path. Phase 5.1 keeps panic
// reporting here (rev2§7 C-M9) even as `Stdout`/`Stderr` move to the console, so a
// distinct type isolates `panic_output` from that change.
struct PanicWriter;

impl io::Write for PanicWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        debug_write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub fn panic_output() -> Option<impl io::Write> {
    Some(PanicWriter)
}
