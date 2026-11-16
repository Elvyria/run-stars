use std::{fs::File, io::Write, os::{fd::AsRawFd, unix::fs::FileExt}};

use memchr::memchr_iter;

use crate::{Task, SPLIT_CHAR};

pub fn write(mut w: impl Write, buffer: &mut Vec<u8>, tasks: &[Task]) -> Result<(), std::io::Error> {
    macro_rules! csv { ($($f:expr),+) => { $( write!(buffer, "{}{SPLIT_CHAR}", $f)?; )+ } }
    buffer.clear();

    for Task { status, code, start, finish, path, output } in tasks.iter() {
        csv!(status, code, start, finish, path.display());

        write_sanitized(&mut *buffer, &output)?;

        buffer.write(b"\n")?;
    }

    w.write_all(buffer)?;
    w.flush()
}

pub fn write_sanitized(mut w: impl Write, s: &str) -> Result<(), std::io::Error> {
    let mut b = s.as_bytes();
    b = b.strip_suffix(b"\n").unwrap_or(b);

    let mut last = 0;

    for pos in memchr_iter(b'\n', b) {
        w.write_all(&b[last..pos])?;
        w.write_all(b"\\n")?;
        last = pos + 1;
    }

    w.write_all(&b[last..])
}

pub enum StateFile {
    File(File),
    Sink,
}

impl StateFile {
    pub fn lock(&self) -> Result<(), std::io::Error> {
        let Self::File(fd) = self else {
            return Ok(())
        };

        let mut lock = libc::flock {
            l_type:   libc::F_WRLCK  as _,
            l_whence: libc::SEEK_END as _,
            l_start:  0,
            l_len:    0,
            l_pid:    0,
        };

        match unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_SETLKW, std::ptr::from_mut(&mut lock)) } {
            -1 => Err(std::io::Error::last_os_error()),
            _  => Ok(()),
        }
    }
}

impl Write for StateFile {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            StateFile::File(fd) => {
                fd.set_len(0)?;
                fd.write_all_at(buf, 0).map(|_| buf.len())
            },
            StateFile::Sink => Ok(buf.len()),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            StateFile::File(fd) => fd.flush(),
            StateFile::Sink => Ok(()),
        }
    }
}

impl Drop for StateFile {
    fn drop(&mut self) {
        if let StateFile::File(fd) = self {
            let _ = fd.sync_all();
        }
    }
}
