use std::cell::UnsafeCell;
use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};

use memchr::memchr_iter;

use crate::error::Error;

const DIR_NAME: &'static str = "run_stars";

#[derive(PartialEq, Eq, Debug)]
pub enum Kind {
    Runtime,
    Persistent,
}

pub fn is_runtime(p: impl AsRef<Path>) -> bool {
    p.as_ref().parent().is_some_and(|p| p == runtime_dir())
}

#[inline]
pub(crate) fn runtime_dir() -> PathBuf {
    PathBuf::from(xdg::runtime()).join(DIR_NAME)
}

#[inline]
pub(crate) fn persistent_dir() -> PathBuf {
    PathBuf::from(xdg::state()).join(DIR_NAME)
}

#[inline]
pub fn init_runtime_dir() -> Result<PathBuf, Error> {
    init_dir(xdg::runtime())
}

#[inline]
pub fn init_persistent_dir() -> Result<PathBuf, Error> {
    init_dir(xdg::state())
}

fn init_dir(p: impl AsRef<Path>) -> Result<PathBuf, Error> {
    let p = p.as_ref();

    match std::fs::metadata(p) {
        Ok(m) if m.is_dir() => {
            let state = p.join(DIR_NAME);

            let Err(io) = std::fs::create_dir(&state) else {
                return Ok(state);
            };

            match io.kind() {
                std::io::ErrorKind::AlreadyExists => Ok(state),
                _ => Err(Error::CreateLocation { path: state.clone(), io}.into()),
            }
        }
        Ok(_) => Err(Error::NotDirectory(p.to_owned()).into()),
        Err(io) => Err(Error::AccessLocation { path: p.to_owned(), io }.into()),
    }
}

const ESCAPE_CHAR: char = '%';

pub fn decode(p: impl Into<OsString>) -> PathBuf {
    let original = p.into().into_vec();
    let c = ESCAPE_CHAR as u8;

    let b = unsafe {
        let mut b = UnsafeCell::new(original);
        let mut chars = memchr_iter(c, &*b.get()).peekable();

        {
            let b = &mut *b.get_mut();
            while let Some(i) = chars.next() {
                if i + 1 != *chars.peek().unwrap_or(&0) {
                    b[i] = b'/';
                } else {
                    chars.next();
                }
            }
        }

        b.into_inner()
    };

    let mut chars = memchr_iter(c, b.as_slice()).peekable();
    if chars.peek().is_some() {
        let mut offset = 0;
        let mut b = b.clone();

        while let Some(i) = chars.next() {
            b.remove(i - offset);
            offset += 1;

            chars.next();
        }

        return PathBuf::from(OsString::from_vec(b))
    }

    PathBuf::from(OsString::from_vec(b))
}

pub fn encode(p: impl Into<OsString>) -> PathBuf {
    let original = p.into().into_vec();
    let c = ESCAPE_CHAR as u8;

    let mut chars = memchr_iter(c, original.as_slice()).peekable();

    let b = match chars.peek().is_none() {
        true  => original,
        false => {
            let mut offset = 0;
            let mut b = original.clone();

            chars.for_each(|i| { b.insert(i + offset, c); offset += 1 });

            b
        }
    };

    unsafe {
        let mut b = UnsafeCell::new(b);

        let chars = memchr_iter(b'/', &*b.get());

        {
            let b = &mut *b.get_mut();
            chars.for_each(|i| b[i] = c);
        }

        PathBuf::from(OsString::from_vec(b.into_inner()))
    }
}
