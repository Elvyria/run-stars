mod error;
mod ls;
mod state;
mod xdg;

use std::{
    fs::File, io::Write, num::NonZeroUsize, os::unix::ffi::OsStrExt, path::PathBuf, process::Stdio
};

use async_process::Command;
use futures_concurrency::{
    future::FutureExt,
    prelude::{ConcurrentStream, IntoConcurrentStream},
};
use futures_lite::future;
use jiff::Timestamp;
use rustix::path::Arg;
use state::{StateFile, Status, StatusMsg, Task};

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

        tasks.push(Task::new(p.clone()));

        (tasks.len() - 1, p, s.clone())
    }).collect();

    drop(s);

    let sum = seahash::hash(args.dir.as_os_str().as_bytes());

    let runtime_path = format!("{}/{sum}", create_runtime_home()?);
    let mut runtime: &mut dyn std::io::Write = match File::create(&runtime_path) {
        Ok(fd) => &mut StateFile(fd),
        Err(_) => &mut std::io::sink(),
    };

    let wait_for_processes = processes.into_co_stream().limit(args.limit).for_each(|(i, p, s)| {
        async move {
            let c = Command::new("/bin/sh")
                .arg(&p)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();

            let status = match c {
                Ok(_) => Status::Running,
                Err(ref e) => {
                    eprintln!("{e}");

                    Status::Failure
                }
            };

            let state = StatusMsg { status, time: Timestamp::now() };
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

            let state = StatusMsg { status, time: Timestamp::now() };
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

    let _ = std::fs::remove_file(runtime_path);

    let state_path = format!("{}/{sum}", create_state_home()?);
    let mut state = File::create(state_path)?;
    state.write_all(&buffer)?;

    Ok(())
}

#[inline]
fn create_home(s: &str) -> Result<String, Error> {
    match std::fs::metadata(s) {
        Ok(m) if m.is_dir() => {
            let state = format!("{}/{}", s, env!("CARGO_CRATE_NAME"));

            let Err(e) = std::fs::create_dir(&state) else {
                return Ok(state);
            };

            match e.kind() {
                std::io::ErrorKind::AlreadyExists => Ok(state),
                _ => Err(e.into()),
            }
        }
        Ok(_) => Err(error::System::NotDirectory(s.to_string()).into()),
        Err(e) => Err(e.into()),
    }
}

fn create_state_home() -> Result<String, Error> {
    create_home(&xdg::state())
}

fn create_runtime_home() -> Result<String, Error> {
    create_home(&xdg::runtime())
}
