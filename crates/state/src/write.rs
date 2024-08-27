use std::{fs::File, io::Write, os::unix::fs::FileExt};

use crate::{Task, SPLIT_CHAR};

// This is &mut dyn std::io::Write without complications.
pub enum StateFile {
    File(File),
    Sink,
}

pub fn write(mut w: impl Write, buffer: &mut Vec<u8>, tasks: &[Task]) -> Result<(), std::io::Error> {
    buffer.clear();

    for task in tasks.iter() {
        write!(buffer, "{}{SPLIT_CHAR}{}{SPLIT_CHAR}{}{SPLIT_CHAR}{}\n",
            task.status,
            task.code,
            task.time,
            task.path.to_string_lossy())?;
    }

    w.write(&buffer)?;
    w.flush()
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
