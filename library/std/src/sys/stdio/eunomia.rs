use crate::io;

// Provided by the seam crate `eunomia-sys` (see `sys/pal/eunomia/mod.rs`). std-port 5.1
// routes ordinary stdout/stdin/stderr over the userspace `user/console` channel (rev2§5.1);
// panic last-words stay on the kernel debug-log (`__eunomia_stdio_write`, rev2§7 C-M9) so
// reporting never depends on the console — which may be the very thing that wedged. All
// marshalling — console chunking/backpressure, the read carry, the debug-log chunking —
// lives in the seam; these arms only delegate.
unsafe extern "Rust" {
    // Console stream I/O (a raw byte pipe): a write is best-effort/infallible (reports the
    // full length written, so `write_all` never loops); a read blocks for the next console
    // message and returns the bytes delivered into `buf`, or `0` at EOF (no console granted).
    fn __eunomia_stdout_write(buf: &[u8]) -> usize;
    fn __eunomia_stderr_write(buf: &[u8]) -> usize;
    fn __eunomia_stdin_read(buf: &mut [u8]) -> usize;
    // Panic last-words: the kernel debug-log (rev2§7).
    fn __eunomia_stdio_write(buf: &[u8]) -> usize;
}

pub struct Stdin;
pub struct Stdout;
pub struct Stderr;

impl Stdin {
    pub const fn new() -> Stdin {
        Stdin
    }
}

// std-port 5.1: real console input. `read` blocks until at least one byte arrives on the
// `stdin` console channel and delivers up to `buf.len()` of it, or returns `0` (EOF) when
// this process was granted no console. The remaining `io::Read` methods use the trait
// defaults over this `read` (a genuine stream, unlike the pre-5.1 EOF stub).
impl io::Read for Stdin {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // SAFETY: the seam fn reads at most `buf.len()` bytes into `buf` (a valid slice)
        // from the console `stdin` channel and returns the count (`0` = EOF).
        Ok(unsafe { __eunomia_stdin_read(buf) })
    }
}

impl Stdout {
    pub const fn new() -> Stdout {
        Stdout
    }
}

impl io::Write for Stdout {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // SAFETY: pure delegation — reads `buf`, marshals it to the console `stdout`
        // channel, and returns the byte count (always `buf.len()`, best-effort/infallible).
        Ok(unsafe { __eunomia_stdout_write(buf) })
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
        // SAFETY: as `Stdout::write`, over the console `stderr` channel (a stream distinct
        // from stdout so diagnostics never enter a pipeline's data).
        Ok(unsafe { __eunomia_stderr_write(buf) })
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

// Separate `Stdin`/`Stdout`/`Stderr` structs (the `motor` shape, not the `unsupported`
// alias) let each stream route independently: stdout/stdin/stderr over the console,
// panic last-words over the debug-log.

pub const STDIN_BUF_SIZE: usize = crate::sys::io::DEFAULT_BUF_SIZE;

pub fn is_ebadf(_err: &io::Error) -> bool {
    // The console and debug-log write paths are infallible, so this is vacuous; `true` is
    // the strictly-safe answer (treat any surfaced error as a missing stream the writer
    // may swallow).
    true
}

// A dedicated writer for panic last-words on the debug-log path (rev2§7 C-M9). std-port
// 5.1 keeps panic reporting here even as `Stdout`/`Stderr` moved to the console, so a
// wedged console cannot swallow a panic; the distinct type isolates `panic_output` from
// that console routing.
struct PanicWriter;

impl io::Write for PanicWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // SAFETY: pure delegation to the debug-log seam fn (reads `buf`, returns the count).
        Ok(unsafe { __eunomia_stdio_write(buf) })
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub fn panic_output() -> Option<impl io::Write> {
    Some(PanicWriter)
}
