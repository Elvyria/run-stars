use std::{fmt::Display, fs::File, io::{self, Write}, os::unix::fs::FileExt, path::PathBuf};

use jiff::Timestamp;
use rustix::path::Arg;

#[derive(Debug)]
pub struct Task {
    pub status: Status,
    pub time:   Timestamp,
    pub path:   PathBuf,
}

pub struct StatusMsg {
    pub status: Status,
    pub time:   Timestamp,
}

impl Task {
    pub fn new(p: PathBuf) -> Self {
        Self {
            status: Status::Waiting,
            time: Timestamp::now(),
            path: p,
        }
    }
}

#[derive(Debug)]
pub enum Status {
    Success,
    Failure,
    Running,
    Waiting,
}

impl Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::fmt::Write;

        let c = match self {
            Status::Success => 'S',
            Status::Failure => 'F',
            Status::Running => 'R',
            Status::Waiting => 'W',
        };

        f.write_char(c)
    }
}

pub fn write(mut w: impl Write, buffer: &mut Vec<u8>, tasks: &[Task]) -> Result<(), std::io::Error> {
    buffer.clear();

    for task in tasks.iter() {
        write!(buffer, "{}:{}:{}\n", task.status, task.time, task.path.to_string_lossy())?;
    }

    w.write(&buffer)?;
    w.flush()
}

pub struct StateFile(pub File);

impl Write for StateFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.set_len(0)?;
        self.0.write_all_at(buf, 0).map(|_| buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}
