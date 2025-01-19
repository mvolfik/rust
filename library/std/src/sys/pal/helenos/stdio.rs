use crate::io;

pub struct Stdin;
pub struct Stdout;
pub struct Stderr;

impl Stdin {
    pub const fn new() -> Stdin {
        Stdin
    }
}

impl io::Read for Stdin {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        Ok(unsafe { libc::fread(buf.as_mut_ptr() as *mut libc::c_void, 1, buf.len(), libc::stdin) })
    }
}

impl Stdout {
    pub const fn new() -> Stdout {
        Stdout
    }
}

impl io::Write for Stdout {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        Ok(unsafe { libc::fwrite(buf.as_ptr() as *const libc::c_void, 1, buf.len(), libc::stdout) })
    }

    fn flush(&mut self) -> io::Result<()> {
        unsafe { libc::fflush(libc::stdout); }
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
        Ok(unsafe { libc::fwrite(buf.as_ptr() as *const libc::c_void, 1, buf.len(), libc::stderr) })
    }

    fn flush(&mut self) -> io::Result<()> {
        unsafe { libc::fflush(libc::stderr); }
        Ok(())
    }
}

pub const STDIN_BUF_SIZE: usize = 0;

pub fn is_ebadf(err: &io::Error) -> bool {
    err.raw_os_error() == Some(libc::EBADF as i32)
}

pub fn panic_output() -> Option<impl io::Write> {
    Some(Stderr::new())
}
