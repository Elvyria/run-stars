use std::{fmt::Debug, path::PathBuf};

use thiserror::Error;

#[derive(Error)]
pub enum Error {
    #[error(transparent)]
    File(#[from] FileError),
}

impl Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

#[derive(Error, Debug)]
pub enum FileError {
    #[error("unable to get an absolute path to a target directory ({path})\n↳ {io}")]
    Absolute {
        io:   std::io::Error,
        path: PathBuf,
    },

    #[error("couldn't create a file ({path}) for the persistant state information\n↳ {io}")]
    CreatePersistant {
        io:   std::io::Error,
        path: PathBuf,
    },

    #[error("couldn't write a persistant state to a file ({path})\n↳ {io}")]
    WritePersistant {
        io:   std::io::Error,
        path: PathBuf,
    },

    #[error("couldn't create an essential directory ({path})\n↳ {io}")]
    CreateLocation {
        io:   std::io::Error,
        path: PathBuf,
    },

    #[error("unable to access a directory ({path})\n↳ {io}")]
    AccessLocation {
        io:   std::io::Error,
        path: PathBuf,
    },

    #[error("runtime state file ({path}) cannot be locked for writing, another instance is probably already running\n↳ {io}")]
    RuntimeLock {
        io:   std::io::Error,
        path: PathBuf,
    },

    #[error("expected a directory at {0}, but {0} is not a directory")]
    NotDirectory(PathBuf),
}
