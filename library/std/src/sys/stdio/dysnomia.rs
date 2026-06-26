use crate::io;
use crate::sys::pal::{abi, count_result};

fn write(function: abi::ConstIoFn, buf: &[u8]) -> io::Result<usize> {
    let mut count = 0;
    let status = unsafe { function(buf.as_ptr(), buf.len() as u64, &mut count) };
    count_result(status, count, buf.len())
}

pub struct Stdin;
pub struct Stdout;
pub struct Stderr;

impl Stdin {
    pub const fn new() -> Stdin {
        Stdin
    }
}

// The remaining `io::Read` methods use the trait defaults.
impl io::Read for Stdin {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut count = 0;
        let status = unsafe {
            abi::__dysnomia_pal_v1_stdin_read(buf.as_mut_ptr(), buf.len() as u64, &mut count)
        };
        count_result(status, count, buf.len())
    }
}

impl Stdout {
    pub const fn new() -> Stdout {
        Stdout
    }
}

impl io::Write for Stdout {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        write(abi::__dysnomia_pal_v1_stdout_write, buf)
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
        write(abi::__dysnomia_pal_v1_stderr_write, buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub const STDIN_BUF_SIZE: usize = crate::sys::io::DEFAULT_BUF_SIZE;

pub fn is_ebadf(_err: &io::Error) -> bool {
    // Treat any surfaced error as a missing stream.
    true
}

struct PanicWriter;

impl io::Write for PanicWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        write(abi::__dysnomia_pal_v1_stdio_write, buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub fn panic_output() -> Option<impl io::Write> {
    Some(PanicWriter)
}
