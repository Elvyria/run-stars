use std::{fmt::Debug, path::PathBuf};

use thiserror::Error;

#[derive(Error)]
pub enum Error {
    #[error("unable to get an absolute path to a target directory ({path})\n{io}")]
    Absolute {
        io:   std::io::Error,
        path: PathBuf,
    },

    #[error("unable to access a directory ({path})\n{io}")]
    AccessLocation {
        io:   std::io::Error,
        path: PathBuf,
    },

    #[error("couldn't create an essential directory ({path})\n{io}")]
    CreateLocation {
        io:   std::io::Error,
        path: PathBuf,
    },

    #[error("unable to open a state file ({path})\n{io}")]
    Open {
        io:   std::io::Error,
        path: PathBuf,
    },

    #[error("couldn't read a line in a state file ({path})\n{io}")]
    Read {
        io:   std::io::Error,
        path: PathBuf,
    },

    #[error("couldn't list state files inside of a directory ({path})\n{io}")]
    ListDir {
        io:   std::io::Error,
        path: PathBuf,
    },

    #[error("couldn't read metadata for a state file ({path})\n{io}")]
    Metadata {
        io:   std::io::Error,
        path: PathBuf,
    },

    #[error("{e:?} in a state file ({path})\nline {num}: \"{line}\"")]
    Parse {
        e:    ParseError,
        num:  usize,
        line: String,
        path: PathBuf,
    },

    #[error("couldn't parse a malformed line in a state file ({path})\nline {n}: \"{line}\"")]
    Malformed {
        n:    usize,
        line: String,
        path: PathBuf,
    },

    #[error("no state file was found, expected at ({0})")]
    NotFound(PathBuf),

    #[error("expected a directory at {0}, but {0} is not a directory")]
    NotDirectory(PathBuf),

    #[error("expected state file ({0}) to be a file")]
    NotFile(PathBuf)
}

#[derive(Error)]
pub enum ParseError {
    #[error("couldn't parse a status key, expected 'S', 'F', 'R', 'W', got '{0}'")]
    Status(String),

    #[error("couldn't parse an exit code, expected 0-255, got '{0}'")]
    Code(String),
    
    #[error("couldn't parse a timestamp, got '{0}'")]
    Timestamp(String),

    #[error("couldn't parse a path to an executable, '{0}' is not a valid path")]
    Path(String),
}

#[derive(Error)]
pub enum LockError {
    #[error("couldn't place a lock on a state file ({path}) for writing\n{io}")]
    Set {
        io: std::io::Error,
        path: PathBuf,
    },

    #[error("unable to get the lock state of a state file ({path})\n{io}")]
    Get {
        io: std::io::Error,
        path: PathBuf,
    }
}

#[inline]
pub fn beautify_newline(s: String) -> String {
    s.replace("\n", "\n↳ ")
}

impl Debug for LockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = beautify_newline(format!("{}", self));
        f.write_str(s.as_str())
    }
}

impl Debug for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = beautify_newline(format!("{}", self));
        f.write_str(s.as_str())
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = beautify_newline(format!("{}", self));
        f.write_str(s.as_str())
    }
}
