use std::ffi::OsStr;
use std::fmt::Display;
use std::io;
use std::os::unix::ffi::OsStringExt;
use std::path::Path;

use futures_lite::{FutureExt, StreamExt};
use futures_time::time::Duration;

use itertools::Itertools;
use jiff::tz::TimeZone;
use jiff::{Span, Timestamp};

use ratatui::Terminal;
use ratatui::backend::Backend;

use state::{State, Status, Task};

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
            time:   {
                task.time.to_zoned(TimeZone::system()).strftime("%a %b %e %I:%M:%S %p").to_string()
            },

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
        ErrorEntry {
            message: format!("{e}"),
            severity
        }
    }
}

impl App {
    pub fn new(p: Option<&Path>) -> Self {
        let mut last_error = None;
        let mut task_entries = Vec::new();
        let mut selection = Selection::StateList;

        // fn tasks() {
        //     match state.tasks() {
        //         Ok((tasks, errors)) => {
        //             task_entries = tasks.into_iter().map(TaskEntry::from).collect();
        //             last_error = errors.last().map(|e| ErrorEntry::new(e, Severity::Low));
        //         },
        //         Err(errors) => {
        //             last_error = errors.last().map(|e| ErrorEntry::new(e, Severity::High));
        //         },
        //     }
        // }

        let mut state_entries: Vec<StateEntry> = state::states().unwrap().into_iter().map(|state| {
            let path = state.path();

            if p.filter(|&p| p == path).is_some() {
                match state.tasks() {
                    Ok((tasks, errors)) => {
                        task_entries = tasks.into_iter().map(TaskEntry::from).collect();
                        last_error = errors.last().map(|e| ErrorEntry::new(e, Severity::Low));
                    },
                    Err(errors) => {
                        last_error = errors.last().map(|e| ErrorEntry::new(e, Severity::High));
                    },
                }

                selection = Selection::TaskTable;
            }

            StateEntry {
                name: path.to_string_lossy().to_string(),
                state,
            }
        }).collect();

        state_entries.sort_by(|a, b| a.name.partial_cmp(&b.name).unwrap());

        if state_entries.len() == 1 {
            selection = Selection::TaskTable;
        }

        if p.is_none() {
            if let Some(entry) = state_entries.first() {
                match entry.state.tasks() {
                    Ok((tasks, errors)) => {
                        task_entries = tasks.into_iter().map(TaskEntry::from).collect();
                        last_error = errors.last().map(|e| ErrorEntry::new(e, Severity::Low));
                    },
                    Err(errors) => {
                        last_error = errors.last().map(|e| ErrorEntry::new(e, Severity::High));
                    },
                }
            }
        }

        Self {
            ui: UI {
                task_table:  TaskTable::new(task_entries.len()),
                state_list:  StateList::new(state_entries.len()),
                selection,
            },

            state_entries,
            task_entries,
            last_error,
        }
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
            entry.state.remove(&state);

            if !(entry.state.runtime || entry.state.persistent) {
                self.state_entries.remove(i);
                self.ui.state_list.len -= 1;
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
        let i = self.ui.state_list.selected();

        let entry = self.state_entries.get_mut(i).unwrap();

        match entry.state.tasks() {
            Ok((tasks, errors)) => {
                self.task_entries = tasks.into_iter().map(TaskEntry::from).collect();
                self.last_error = errors.last().map(|e| ErrorEntry::new(e, Severity::Low));
            },
            Err(errors) => {
                self.task_entries = Vec::new();
                self.last_error = errors.last().map(|e| ErrorEntry::new(e, Severity::High));
            },
        }

        self.ui.task_table.set_len(self.task_entries.len());
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
    let mut fs_events = state::monitor::monitor().unwrap();

    // TODO: Recalculate timeout if caught an event
    let timeout = Duration::from_millis(100);
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
