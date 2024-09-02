use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use run_stars_lib::{monitor::{self, StateEvent}, path, State};

use crate::app::{Action, App};

pub trait Handler {
    fn handle_fs(&mut self, event: Option<StateEvent>) -> Action;
    fn handle_ui(&mut self, event: Option<Result<crossterm::event::Event, std::io::Error>>) -> Action;
    fn handle_keyboard(&mut self, key: KeyEvent) -> Action;
}

impl Handler for App {
    fn handle_fs(&mut self, event: Option<StateEvent>) -> Action {
        let Some(io_event) = event else {
            return Action::Tick
        };

        match io_event.event {
            monitor::Event::Modified => {
                match self.is_selected_state(io_event.file_name.as_os_str()) {
                    true  => Action::RefreshTasks,
                    false => Action::Tick,
                }
            },
            monitor::Event::New => {
                match io_event.kind {
                    path::Kind::Runtime => Action::AddState(State::new(io_event.file_name).runtime().running()),
                    path::Kind::Persistent => Action::AddState(State::new(io_event.file_name).persistent()),
                }
            },
            monitor::Event::Removed  => {
                match io_event.kind {
                    path::Kind::Runtime => Action::RemoveState(State::new(io_event.file_name).runtime().running()),
                    path::Kind::Persistent => Action::RemoveState(State::new(io_event.file_name).persistent()),
                }
            },
            monitor::Event::Closed => {
                match io_event.kind {
                    path::Kind::Runtime => Action::RemoveState(State::new(io_event.file_name).running()),
                    _                   => Action::Tick,
                }
            }
        }
    }

    fn handle_ui(&mut self, event: Option<Result<crossterm::event::Event, std::io::Error>>) -> Action {
        use crossterm::event::Event;

        let Some(event) = event else {
            return Action::Tick
        };

        match event.unwrap() {
            Event::Key(key) => self.handle_keyboard(key),
            _               => Action::Tick,
        }
    }

    fn handle_keyboard(&mut self, key: KeyEvent) -> Action {
        if key.kind == KeyEventKind::Press {
            return match key.code {
                KeyCode::Char('q') | KeyCode::Esc   => self.quit(),
                KeyCode::Char('j') | KeyCode::Down  => self.ui.next(),
                KeyCode::Char('k') | KeyCode::Up    => self.ui.previous(),
                KeyCode::Char('h') | KeyCode::Left  => self.ui.focus_state_list(),
                KeyCode::Char('l') | KeyCode::Right => self.ui.focus_task_table(),
                KeyCode::Tab                        => self.ui.switch(),
                _                                   => Action::Tick
            }
        }

        Action::Tick
    }
}
