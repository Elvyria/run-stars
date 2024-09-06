# run-stars

A way to run cron jobs in parallel and monitor their state.
(run-parts replacement, with an ability to create a WEB/TUI/GUI clients that show job states)

🚧 README, LICENSE 🚧

## Building
To build this little thing, you'll need some [Rust](https://www.rust-lang.org/).

```sh
git clone --depth 1 https://github.com/Elvyria/run-stars
cd run-stars
cargo build --locked --release
```

## Locations
To not torture the persistent storage without a need, the runner first writes the state in a runtime directory, that usually points to a part in random access memory.
```sh
/run/run_stars/...
```
or
```sh
${XDG_RUNTIME_DIR:-/run/user/$UID}/run_stars/...
```
While tasks are running, the state file is locked with a [fnctl²](https://man7.org/linux/man-pages/man2/fcntl.2.html) write lock to protect ourselves against dangling states that might be leftover if something happens to the runner and it's unable to report the final state. 

When all tasks are completed, the runner writes the final state to the persistent storage and removes temporary state from the runtime location.
```sh
/var/lib/run_stars/...
```

```sh
${XDG_STATE_HOME:-$HOME/.local/state}/run_stars/...
```

## Format
Runner reports the state of an each running task in a simple, human-readable fashion.
```csv
S,0,2024-09-06T03:33:08.612671265Z,/etc/cron.weekly/cleanup
```

Or

```csv
[S],[C],[T],[P]
```
#### [S] : A single ASCII character that represents the state of a task
- `S` - Success
- `F` - Failure
- `R` - Running
- `W` - Waiting
- `U` - Unknown

#### [C] : An exit code or 0 if the task is still running
- `0-255`

#### [T] : A timestamp in ISO 8601 at the moment when task was started or exited
- `2024-09-06T03:33:08.612671265Z`

#### [P] : An absolute path to the executable
- `/etc/cron.weekly/cleanup`
