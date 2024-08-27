pub mod error;
pub mod path;
// pub mod read;
pub mod write;
pub mod monitor;

use std::ffi::OsString;
use std::fmt::Display;
use std::fs::{DirEntry, File, FileType};
use std::io::{BufRead, BufReader, ErrorKind};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::SystemTime;

use jiff::Timestamp;
use memchr::memchr_iter;

use error::{Error, ParseError};

#[derive(Debug)]
pub struct State {
    pub file_name:  OsString,
    pub persistent: bool,
    pub runtime:    bool,
    pub running:    bool,
}

impl PartialEq for State {
    fn eq(&self, other: &Self) -> bool {
        self.file_name == other.file_name
    }
}

impl State {
    pub fn new_runtime(s: OsString) -> Self {
        State {
            file_name:  s,
            persistent: false,
            runtime:    true,
            running:    false,
        }
    }

    pub fn new_persistent(s: OsString) -> Self {
        State {
            file_name:  s,
            persistent: true,
            runtime:    false,
            running:    false,
        }
    }

    pub fn add(&mut self, other: &State) {
        self.persistent |= other.persistent;
        self.runtime    |= other.runtime;
        self.running    |= other.running;
    }

    pub fn remove(&mut self, other: &State) {
        self.persistent ^= other.persistent;
        self.runtime    ^= other.runtime;
        self.running    ^= other.running;
    }

    pub fn path(&self) -> PathBuf {
        path::decode(self.file_name.clone())
    }

    pub fn has_persistent(&self) -> bool {
        let p = path::persistent_dir().join(&self.file_name);

        std::fs::metadata(p).is_ok_and(|meta| meta.is_file())
    }

    pub fn exists(&self) -> bool {
        let runtime_path = path::runtime_dir().join(&self.file_name);
        let persistent_path = path::persistent_dir().join(&self.file_name);

        for p in [runtime_path, persistent_path].into_iter() {
            let meta = std::fs::metadata(p);

            if meta.is_ok_and(|meta| meta.is_file()) {
                return true
            }
        }

        false
    }

    pub fn tasks(&self) -> Result<(Vec<Task>, Vec<Error>), Vec<Error>> {
        let mut current: Option<(PathBuf, SystemTime)> = None;
        let mut errors = Vec::new();

        let runtime_path = path::runtime_dir().join(&self.file_name);
        let persistent_path = path::persistent_dir().join(&self.file_name);

        let paths = [runtime_path, persistent_path];

        for p in paths.into_iter() {
            match std::fs::metadata(&p) {
               Ok(meta) if meta.is_file() => {
                    let p_time = meta.modified().expect("modified field must be available to decide which state to read");

                    if current.as_ref().map_or(true, |(_, current_time)| *current_time < p_time) {
                        current = Some((p, p_time));
                    }
                },
                Err(io) if io.kind() == ErrorKind::NotFound => {
                    errors.push(Error::NotFound(p));
                },
                Err(io) => {
                    errors.push(Error::Metadata { path: p.clone(), io });
                },
                Ok(_) => {
                    errors.push(Error::NotFile(p.clone()));
                },
            };
        }

        if let Some((p, _)) = current {
            match parse(&p) {
                Ok(tasks) => return Ok((tasks, errors)),
                Err(e) => errors.push(e),
            }
        }

        Err(errors)
    }
}

pub enum Directory {
    Runtime,
    Persistent,
}

pub struct StateChange {
    pub status: Status,
    pub code:   u8,
    pub time:   Timestamp,
}

pub struct Task {
    pub status: Status,
    pub code:   u8,
    pub time:   Timestamp,
    pub path:   PathBuf,
}

impl Task {
    pub fn new(p: PathBuf) -> Self {
        Self {
            status: Status::Waiting,
            code:   0,
            time:   Timestamp::now(),
            path:   p,
        }
    }
}

#[derive(PartialEq)]
pub enum Status {
    Success,
    Failure,
    Running,
    Waiting,
    Unknown,
}

impl Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::fmt::Write;

        let c = match self {
            Status::Success => 'S',
            Status::Failure => 'F',
            Status::Running => 'R',
            Status::Waiting => 'W',
            Status::Unknown => 'U',
        };

        f.write_char(c)
    }
}

impl FromStr for Status {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "S" => Ok(Status::Success),
            "F" => Ok(Status::Failure),
            "R" => Ok(Status::Running),
            "W" => Ok(Status::Waiting),
            "U" => Ok(Status::Unknown),
            _   => Err(ParseError::Status(s.to_owned())),
        }
    }
}

pub fn states() -> Result<Vec<State>, Error> {
    fn list_files(path: PathBuf) -> Result<impl Iterator<Item = DirEntry>, Error> {
        Ok(std::fs::read_dir(&path)
            .map_err(|io| Error::ListDir { path, io })?
            .flatten()
            .filter(|f| f.file_type().as_ref().is_ok_and(FileType::is_file)))
    }

    let runtime_path = path::init_runtime_dir()?;

    let mut states: Vec<_> = list_files(runtime_path)?
        .map(|f| {
            let mut state = State::new_runtime(f.file_name());

            // TODO: LCK feature
            if false {
                state.running = true;
            }

            state
        })
        .collect();

    let persistent_path = path::init_persistent_dir()?;

    list_files(persistent_path)?
        .for_each(|f| match states.iter_mut().find(|state| state.file_name == f.file_name()) {
            Some(state) => state.persistent = true,
            None => {
                let state = State::new_persistent(f.file_name());
                states.push(state);
            }
        });

    Ok(states)
}

pub const SPLIT_CHAR: char = ',';

fn parse(p: impl AsRef<Path> + Copy) -> Result<Vec<Task>, Error> {
    let p = p.as_ref();

    if !p.is_file() {
        return Err(Error::NotFound(p.to_owned()));
    }

    let fd = File::open(p).map_err(|io| Error::Open { path: p.to_owned(), io })?;
    let reader = BufReader::new(fd);

    let mut v = Vec::new();

    for (i, l) in reader.lines().enumerate() {
        let l = l.map_err(|io| Error::Read { path: p.to_owned(), io })?;
        let mut parts = memchr_iter(SPLIT_CHAR as u8, l.as_bytes());

        let malformed_err = || Error::Malformed { n: i + 1, line: l.to_owned(), path: p.to_owned() };

        let status = parts.next().ok_or_else(malformed_err)?;
        let code = parts.next().ok_or_else(malformed_err)?;
        let time = parts.next().ok_or_else(malformed_err)?;

        // SAFETY: memchr_iter returns values inside of a slice 
        let s_status = unsafe { l.get_unchecked(..status) };
        let s_code   = unsafe { l.get_unchecked(status + 1..code) };
        let s_time   = unsafe { l.get_unchecked(code + 1..time) };
        let s_path   = unsafe { l.get_unchecked(time + 1..) };

        let parse_err = |e: ParseError| Error::Parse {
            e,
            num:  i,
            line: l.clone(),
            path: p.to_owned()
        };

        let status = Status::from_str(s_status).map_err(|_| parse_err(ParseError::Status(s_status.to_owned())))?;
        let code = u8::from_str(s_code).map_err(|_| parse_err(ParseError::Code(s_code.to_owned())))?;
        let time = Timestamp::from_str(s_time).map_err(|_| parse_err(ParseError::Timestamp(s_time.to_owned())))?;
        let path = PathBuf::from_str(s_path).map_err(|_| parse_err(ParseError::Path(s_path.to_owned())))?;

        v.push(Task { status, code, time, path });
    }

    Ok(v)
}
