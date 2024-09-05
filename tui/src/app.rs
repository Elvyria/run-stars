use std::io;
use std::ffi::OsStr;
use std::fmt::Display;
use std::os::unix::ffi::OsStringExt;

use futures_lite::{FutureExt, StreamExt};
use futures_time::time::Duration;

use itertools::Itertools;

use jiff::tz::TimeZone;
use jiff::{Span, Timestamp};

use ratatui::Terminal;
use ratatui::backend::Backend;

use run_stars_lib::error::Error;
use run_stars_lib::{State, Status, Task};

use crate::handler::Handler;
use crate::render::{render, Selection, StateList, TaskTable, UI};
use crate::spinner::{self, Spinner};

pub struct App {
    pub ui: UI,

    pub state_entries: Vec<StateEntry>, 
    pub task_entries:  Vec<TaskEntry>,
    pub last_error:    Option<ErrorEntry>,
}

pub struct StateEntry {
    pub name: String,
    pub state: State,
}

impl From<State> for StateEntry {
    fn from(state: State) -> Self {
        StateEntry {
            name: state.path().to_string_lossy().to_string(),
            state,
        }
    }
}

pub struct TaskEntry {
    pub status:  Status,
    pub path:    String,
    pub time:    String,

    pub spinner: Spinner,
}

pub enum Action {
    Tick,
    AddState(State),
    RemoveState(State),
    RefreshTasks,
    Quit,
}

impl From<Task> for TaskEntry {
    fn from(task: Task) -> Self {
        TaskEntry {
            status: task.status,
            path:   unsafe {
                let v = task.path.into_os_string().into_vec();
                String::from_utf8_unchecked(v)
            },
            time: task.time.to_zoned(TimeZone::system()).strftime("%a %b %e %I:%M:%S %p").to_string(),
            spinner: Spinner::new(spinner::BRAILE),
        }
    }
}

pub enum Severity {
    High,
    Low,
}

pub struct ErrorEntry {
    pub message:  String,
    pub severity: Severity,
}

impl ErrorEntry {
    fn new(e: impl Display, severity: Severity) -> Self {
        let mut message = format!("{e}");

        if let Some(c) = message.get_mut(..1) {
            c.make_ascii_uppercase();
        }

        ErrorEntry {
            message,
            severity
        }
    }
}

impl App {
    pub fn new(dir: Option<String>) -> Result<Self, Error> {
        let mut selection = Selection::StateList;

        let mut state_entries: Vec<_> = run_stars_lib::states()?
            .into_iter()
            .map(|state| StateEntry {
                name: state.path().to_string_lossy().to_string(),
                state,
            })
            .collect();

        state_entries.sort_by(|a, b| a.name.partial_cmp(&b.name).unwrap());

        if state_entries.len() == 1 {
            selection = Selection::TaskTable;
        }

        let mut app = Self {
            ui: UI {
                task_table:  TaskTable::new(0),
                state_list:  StateList::new(state_entries.len()),
                selection,
            },

            state_entries,
            task_entries: Vec::new(),
            last_error:  None,
        };

        if let Some(dir) = dir {
            app.state_entries.iter()
                .position(|entry| entry.name == dir)
                .map(|i| app.ui.state_list.select(i));
        }

        app.refresh_tasks();

        Ok(app)
    }

    fn add_state_unchecked(&mut self, state: State) {
        let existing = self.state_entries.iter_mut()
            .find(|entry| entry.state == state);

        match existing {
            Some(entry) => entry.state.add(&state),
            None => {
                // TODO: Sort and make sure that selection wasn't moved
                self.state_entries.push(state.into());
                // self.state_entries.sort_by(|a, b| a.file_name.(b.file_name));
                self.ui.state_list.len += 1;
            },
        } 
    }

    fn remove_state_unchecked(&mut self, state: State) {
        let existing = self.state_entries.iter_mut()
            .find_position(|entry| entry.state == state);

        if let Some((i, entry)) = existing {
            entry.state.sub(&state);

            match entry.state.exists() {
                true => if state.running {
                    self.task_entries.iter_mut().for_each(|e| {
                        if e.status == Status::Running {
                            e.status = Status::Unknown
                        }
                    })
                }
                false => {
                    self.state_entries.remove(i);
                    self.ui.state_list.len -= 1;

                    if i == self.state_entries.len() {
                        self.ui.state_list.previous();
                    }

                    self.refresh_tasks();
                }
            }
        } 
    }

    pub fn selected_state(&self) -> Option<&StateEntry> {
        self.state_entries.get(self.ui.state_list.selected())
    }

    pub fn is_selected_state(&self, file_name: &OsStr) -> bool {
        self.selected_state().is_some_and(|entry| entry.state.file_name == file_name)
    }

    fn refresh_tasks(&mut self) {
        self.task_entries.clear();

        let Some(entry) = self.selected_state() else {
            return
        };

        let running = entry.state.runtime && entry.state.running;

        match entry.state.tasks() {
            Ok((tasks, errors)) => {
                self.task_entries.extend(tasks.into_iter().map(|mut task| {
                    if task.status == Status::Running && !running  {
                        task.status = Status::Unknown;
                    }

                    TaskEntry::from(task)
                }));

                self.set_error(errors.last(), Severity::Low);
            },
            Err(errors) => {
                self.set_error(errors.last(), Severity::High);
            },
        }

        self.ui.task_table.set_len(self.task_entries.len());
    }

    pub fn set_error(&mut self, e: Option<impl Display>, severity: Severity) {
        self.last_error = e.map(|e| ErrorEntry::new(e, severity));
    }

    pub fn quit(&mut self) -> Action {
        Action::Quit
    }
}

pub enum Event<FS, UI> {
    FS(FS),
    UI(UI),
}

pub async fn run<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    use futures_time::prelude::*;

    let mut ui_events = crossterm::event::EventStream::new();
    let mut fs_events = run_stars_lib::monitor::monitor().unwrap();

    let mut timeout: Duration = Duration::from_millis(100);
    let mut interval = Interval::new(timeout.into());

    loop {
        terminal.draw(|f| render(f, &mut app, interval.is_late()))?;

        let fs_events = async { Event::FS(fs_events.next().await) };
        let ui_events = async { Event::UI(ui_events.next().await) };

        let event = match fs_events.or(ui_events).timeout(timeout).await {
            Ok(event) => event,
            Err(_) => continue,
        };

        let action = match event {
            Event::FS(fs_event) => app.handle_fs(fs_event),
            Event::UI(ui_event) => app.handle_ui(ui_event),
        };

        match action {
            Action::Tick => {},
            Action::RefreshTasks => app.refresh_tasks(),
            Action::AddState(state) => app.add_state_unchecked(state),
            Action::RemoveState(state) => app.remove_state_unchecked(state),
            Action::Quit => break,
        }

        timeout = interval.delta().into();
    }

    Ok(())
}

struct Interval {
    baseline: Timestamp,
    delta:    Span,
}

impl Interval {
    fn new(delta: std::time::Duration) -> Self {
        Interval {
            baseline: Timestamp::now(),
            delta: Span::try_from(delta).expect("this is a compile time value that should be \"correct\""),
        }
    }

    fn delta(&self) -> std::time::Duration {
        let now = Timestamp::now();

        // SAFETY: jiff - the time library guarantees that default Timestamp configuration doesn't cause errors
        let since = unsafe { now.since(self.baseline).unwrap_unchecked() };

        // SAFETY: self.delta converted from std::time::Duration in Self::new
        self.delta.checked_sub(since)
            .and_then(std::time::Duration::try_from)
            .unwrap_or_else(|_| unsafe { self.delta.try_into().unwrap_unchecked() })
    }

    fn is_late(&mut self) -> bool {
        let now = Timestamp::now();

        // SAFETY: jiff - the time library guarantees that default Timestamp configuration doesn't cause errors
        let since = unsafe { now.since(self.baseline).unwrap_unchecked() };

        // SAFETY: Span must be tested after compiling to ensure safety
        let compare = unsafe { since.compare(self.delta).unwrap_unchecked() };

        if compare.is_ge() {
            self.baseline = now;

            return true
        }

        false
    }
}
