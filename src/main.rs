mod error;
mod ls;
mod state;
mod xdg;

use std::{
    cell::UnsafeCell, ffi::OsString, fs::File, io::Write, num::NonZeroUsize, os::unix::ffi::OsStringExt, path::{self, Path, PathBuf}, process::Stdio
};

use async_process::Command;
use futures_concurrency::{
    future::FutureExt,
    prelude::{ConcurrentStream, IntoConcurrentStream},
};
use futures_lite::future;
use jiff::Timestamp;
use memchr::memchr_iter;
use rustix::path::Arg;
use state::{StateFile, Status, StateChange, Task};

use error::Error;

#[derive(argh::FromArgs)]
/// Args
struct Args {
    /// directory
    #[argh(positional)]
    dir: PathBuf,

    /// list items
    #[argh(switch)]
    list: bool,

    /// limit 
    #[argh(option)]
    limit: Option<NonZeroUsize>,
}

fn main() -> Result<(), Error> {
    let args: Args = argh::from_env();

    let files = ls::files(&args.dir).unwrap();

    if args.list {
        files.for_each(|f| println!("{}", f.path().to_string_lossy()));

        return Ok(())
    }

    let mut tasks = vec![];

    let (s, r) = async_channel::unbounded();
    let processes: Vec<_> = files.map(|f| {
        let p = f.path();

        let mut c = Command::new("/bin/sh"); c
                            .stdout(Stdio::null())
                            .stderr(Stdio::null())
                            .arg(&p);

        tasks.push(Task::new(p));

        (tasks.len() - 1, c, s.clone())
    }).collect();

    drop(s);

    let target_dir = escape_path(path::absolute(args.dir).unwrap(), '%');

    let mut runtime_path = create_runtime_home()?;
    runtime_path.push(&target_dir);

    let mut runtime: &mut dyn std::io::Write = match File::create(&runtime_path) {
        Ok(fd) => &mut StateFile(fd),
        Err(_) => &mut std::io::sink(),
    };

    let wait_for_processes = processes.into_co_stream().limit(args.limit).for_each(|(i, mut c, s)| {
        async move {
            let c = c.spawn();

            let status = match c {
                Ok(_) => Status::Running,
                Err(ref e) => {
                    eprintln!("{e}");

                    Status::Failure
                }
            };

            let state = StateChange { status, time: Timestamp::now() };
            s.send_blocking((i, state)).unwrap();

            let Ok(mut child) = c else { return };

            let status = match child.status().await {
                Ok(code) if code.success() => Status::Success,
                Ok(_) => Status::Failure,
                Err(e) => {
                    eprintln!("{e}");

                    Status::Failure
                }
            };

            let state = StateChange { status, time: Timestamp::now() };
            s.send_blocking((i, state)).unwrap();
        }
    });

    let write_state = async {
        let mut buffer = vec![];

        while let Ok(mut msg) = r.recv().await {
            loop {
                let (i, state) = msg;

                let t = &mut tasks[i];
                t.status = state.status;
                t.time = state.time;

                msg = match r.try_recv() {
                    Ok(msg) => msg,
                    Err(_) => break,
                }
            }

            if let Err(e) = state::write(&mut runtime, &mut buffer, &tasks) {
                eprintln!("{e}");
            }
        }

        buffer
    };

    let (_, buffer) = future::block_on(wait_for_processes.join(write_state));

    let _ = std::fs::remove_file(&runtime_path);

    write_persistant_state(&buffer, target_dir)?;

    Ok(())
}

fn write_persistant_state(b: &[u8], target: impl AsRef<Path>) -> Result<(), Error> {
    let mut state_path = create_state_home()?;
    state_path.push(target);

    let mut state = File::create(state_path)?;
    state.write_all(&b).map_err(Into::into)
}

fn escape_path(p: PathBuf, c: char) -> PathBuf {
    let original = p.into_os_string().into_vec();
    let c = c as u8;

    let b = {
        let mut pos = memchr_iter(c, original.as_slice());
        match pos.next() {
            Some(i) => {
                let mut b = original.clone();
                b.insert(i, c);

                let mut offset = 1;

                pos.for_each(|i| { b.insert(i + offset, c); offset += 1 });

                b
            }
            None => original,
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

fn create_home(p: impl AsRef<Path>) -> Result<PathBuf, Error> {
    let p = p.as_ref();

    match std::fs::metadata(p) {
        Ok(m) if m.is_dir() => {
            let mut state = p.to_owned();
            state.push(env!("CARGO_CRATE_NAME"));

            let Err(e) = std::fs::create_dir(&state) else {
                return Ok(state);
            };

            match e.kind() {
                std::io::ErrorKind::AlreadyExists => Ok(state),
                _ => Err(e.into()),
            }
        }
        Ok(_) => Err(error::System::NotDirectory(p.to_string_lossy().to_string()).into()),
        Err(e) => Err(e.into()),
    }
}

fn create_state_home() -> Result<PathBuf, Error> {
    create_home(&xdg::state())
}

fn create_runtime_home() -> Result<PathBuf, Error> {
    create_home(&xdg::runtime())
}
