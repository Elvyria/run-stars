use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    System(#[from] System),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Error, Debug)]
pub enum System {
    #[error("{0}: is not a directory")]
    NotDirectory(String)
}
