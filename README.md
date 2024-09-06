# run-stars
#### " How are my cron jobs doing? Are they even working? Did something fail? "

Did you know that cron and similar projects are not actually responsible for running files that reside in:
- `/etc/cron.daily`
- `/etc/cron.weekly`
- ...

What actually runs them is usually a [run-parts⁸](https://manpages.ubuntu.com/manpages/focal/en/man8/run-parts.8.html).  
But it would be good to know if something has failed, right?  
Because if we don't know, we might as well just not run our tasks at all.

Now there's a simple way to know! Featuring a [run-parts⁸](https://manpages.ubuntu.com/manpages/focal/en/man8/run-parts.8.html) replacement with a beautiful TUI.

## Features
- Active job monitoring.
- Simple file oriented design with a human-readable format.
- Parallel execution.
- Freedom to choose your own front-end for monitoring.

## Usage
This projects comes with two binaries:
- Runner - `run-stars`
- TUI - ``run-stars-tui``

Runner is responsible for execution of files in a given directory and updates to the correlating state:
```sh
(sudo) run-stars -- /etc/cron.daily
```
By default the runner will execute all files at the same time, but this behavior can be restricted with a `--limit` flag, which will limit the amount of tasks that run simultaneously:
```sh
(sudo) run-stars --limit 1 -- /etc/cron.daily
```


TUI on the other hand provides a comfortable way of monitoring all running, finished and dangling states that runner reports:
```sh
(sudo) run-stars-tui -- [Optional Directory: /etc/cron.daily]
```

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
