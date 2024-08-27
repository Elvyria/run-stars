mod error;
mod ls;

use std::fs::File;
use std::io::Write;
use std::num::NonZeroUsize;
use std::path::{self, Path, PathBuf};
use std::process::Stdio;

use async_process::Command;
use fs4::fs_std::FileExt;
use futures_concurrency::future::FutureExt;
use futures_concurrency::prelude::{ConcurrentStream, IntoConcurrentStream};
use futures_lite::future;
use jiff::Timestamp;
use rustix::path::Arg;

use state::{Status, StateChange, Task};
use state::write::StateFile;

use error::{Error, FileError};

#[derive(argh::FromArgs)]
/// Batch executor with a convenient state reporting.
struct Args {
    /// directory that contains to be executed files
    #[argh(positional)]
    dir: PathBuf,

    /// print a relative path to an each file that will be executed
    #[argh(switch)]
    list: bool,

    /// limit the amount of simultaneously running tasks
    #[argh(option)]
    limit: Option<NonZeroUsize>,

    /// reverse order of execution
    #[argh(switch)]
    reverse: bool,
}

fn main() -> Result<(), Error> {
    let args: Args = argh::from_env();

    let mut target_dir = path::absolute(&args.dir)
        .map_err(|io| FileError::Absolute { path: args.dir, io })?;

    let files = ls::files(&target_dir)
        .map_err(|io| FileError::AccessLocation { path: target_dir.clone(), io })?;

    let mut tasks = vec![];

    let (s, r) = async_channel::unbounded();
    let mut processes: Vec<_> = files.map(|f| {
        let p = f.path();

        tasks.push(Task::new(p.clone()));

        (tasks.len() - 1, p, s.clone())
    }).collect();

    drop(s);

    processes.sort_by(|a, b| a.1.cmp(&b.1));

    if args.reverse {
        processes.reverse();
    }

    if args.list {
        processes.iter().for_each(|(_, p, _)| println!("{}", p.to_string_lossy()));

        return Ok(())
    }

    target_dir = state::path::encode(target_dir);

    let mut runtime_path = state::path::init_runtime_dir()?;
    runtime_path.push(&target_dir);

    let mut runtime: StateFile = match File::create(&runtime_path) {
        Ok(fd) => {
            fd.try_lock_exclusive().map_err(|io| FileError::RuntimeLock { path: runtime_path.clone(), io })?;
            StateFile::File(fd)
        },
        Err(_) => StateFile::Sink,
    };

    let wait_for_processes = processes.into_co_stream().limit(args.limit).for_each(|(i, p, s)| {
        async move {
            let handle_error = |e: &std::io::Error| {
                eprintln!("{}: {e}", &p.to_string_lossy());
            };

            let c = Command::new(&p)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .inspect_err(handle_error);

            let (status, code) = match c {
                Ok(_)  => (Status::Running, 0),
                Err(_) => (Status::Failure, 1),
            };

            let state = StateChange { status, code, time: Timestamp::now() };
            s.send_blocking((i, state)).unwrap();

            let Ok(mut child) = c else { return };

            let status = child.status().await
                .inspect_err(handle_error);

            let (status, code) = match status {
                Ok(status) if status.success() => (Status::Success, 0),
                Ok(status) => (Status::Failure, status.code().unwrap_or(1)),
                Err(_) => (Status::Failure, 1),
            };

            let state = StateChange { status, code: code as u8, time: Timestamp::now() };
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

            if let Err(e) = state::write::write(&mut runtime, &mut buffer, &tasks) {
                eprintln!("{e}");
            }
        }

        buffer
    };

    let (_, buffer) = future::block_on(wait_for_processes.join(write_state));

    drop(runtime);

    write_persistant_state(&buffer, target_dir)?;

    let _ = std::fs::remove_file(&runtime_path);

    Ok(())
}

fn write_persistant_state(b: &[u8], target: impl AsRef<Path>) -> Result<(), Error> {
    let mut state_path = state::path::init_persistent_dir()?;
    state_path.push(target);

    let mut state = File::create(&state_path)
        .map_err(|io| FileError::CreatePersistant { path: state_path.clone(), io })?;

    state.write_all(&b)
        .map_err(|io| FileError::WritePersistant { path: state_path.clone(), io })?;

    state.sync_all()
        .map_err(|io| FileError::SyncPersistant { path: state_path, io }.into())
}
