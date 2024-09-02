use std::{fmt::Debug, path::PathBuf};

use run_stars_lib::error;
use thiserror::Error;

#[derive(Error)]
pub enum Error {
    #[error(transparent)]
    File(#[from] FileError),

    #[error(transparent)]
    State(#[from] run_stars_lib::error::Error),
}

impl Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = error::beautify_newline(format!("{}", self));
        f.write_str(s.as_str())
    }
}

#[derive(Error, Debug)]
pub enum FileError {
    #[error("unable to get an absolute path to the target directory ({path})\n{io}")]
    Absolute {
        io:   std::io::Error,
        path: PathBuf,
    },

    #[error("unable to access a directory ({path})\n{io}")]
    AccessLocation {
        io:   std::io::Error,
        path: PathBuf,
    },

    #[error("couldn't create a file ({path}) for the persistant state information\n{io}")]
    CreatePersistant {
        io:   std::io::Error,
        path: PathBuf,
    },

    #[error("couldn't write a persistant state to a file ({path})\n{io}")]
    WritePersistant {
        io:   std::io::Error,
        path: PathBuf,
    },

    #[error("couldn't sync the persistant state file to the location ({path})\n{io}")]
    SyncPersistant {
        io:   std::io::Error,
        path: PathBuf,
    },
}
