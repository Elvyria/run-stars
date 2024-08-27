use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Flex, Layout, Margin, Rect};
use ratatui::style::{palette::tailwind, Style, Stylize};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::block::{Position, Title};
use ratatui::widgets::{Block, BorderType, Cell, HighlightSpacing, List, ListItem, ListState, Padding, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table, TableState, Wrap};

use state::Status;

use crate::app::{Action, App, ErrorEntry, Severity, TaskEntry};
use crate::spinner::{self, Spinner};

mod theme {
    use ratatui::style::Color;
    use ratatui::style::palette::tailwind;

    pub const COLOR_BLOCK_TITLE: Color = tailwind::GRAY.c500;
    pub const COLOR_BORDER: Color = tailwind::GRAY.c800;
    pub const COLOR_ERROR: Color = tailwind::LIME.c700;
    pub const COLOR_FOREGROUND: Color = tailwind::SLATE.c200;
    pub const COLOR_SELECTION: Color = tailwind::GRAY.c600;
    pub const COLOR_SELECTION_FOCUSED: Color = tailwind::PURPLE.c500;
}

pub fn render(f: &mut Frame, app: &mut App, tick: bool) {
    let outer_layout = Layout::horizontal([
        Constraint::Percentage(15),
        Constraint::Percentage(85),
    ])
    .split(f.size());

    let task_panel_layout = Layout::vertical([ Constraint::Min(5), ])
    .split(outer_layout[1]);

    render_state_list(f, app, outer_layout[0], tick);
    render_task_table(f, app, task_panel_layout[0], tick);
    render_scrollbar(f, &mut app.ui.task_table.scroll, task_panel_layout[0]);
    render_error(f, task_panel_layout[0], &app.last_error);
}

fn render_state_list(f: &mut Frame, app: &mut App, area: Rect, tick: bool) {
    let mut style_selected: Style = Style::new()
        .fg(theme::COLOR_SELECTION);

    if app.ui.selection == Selection::StateList {
        style_selected = style_selected.fg(theme::COLOR_SELECTION_FOCUSED);
    }

    let entries = app.state_entries.iter()
        .map(|f| {
            let mut item = ListItem::new(f.name.as_str());
            if f.state.runtime {
                let mut text = Text::from(app.ui.state_list.spinner.current().fg(tailwind::INDIGO.c500));

                text.push_span(Span::raw(" "));
                text.push_span(Span::raw(&f.name));

                item = ListItem::new(text);

                if tick {
                    app.ui.state_list.spinner.next();
                }
            }
            item
        });

    let border = Block::bordered()
        .padding(Padding::uniform(1))
        .border_type(BorderType::Rounded).fg(theme::COLOR_BORDER);

    let l = List::new(entries)
    .highlight_style(style_selected)
    .highlight_spacing(HighlightSpacing::WhenSelected)
    .block(border)
    .fg(theme::COLOR_FOREGROUND);

    f.render_stateful_widget(l, area, &mut app.ui.state_list.state);
    // f.render_widget(Line::from("Directories").bg(tailwind::INDIGO.c600).fg(tailwind::WHITE), area);
}

fn table_header() -> Row<'static> {
    ["", "Time", "Path", "Message"]
        .into_iter()
        .map(Cell::from)
        .collect::<Row>()
        .fg(theme::COLOR_FOREGROUND)
        .height(2)
}

fn status(entry: &TaskEntry) -> Span {
    match entry.status {
        state::Status::Success => "✓".fg(tailwind::GREEN.c400),
        state::Status::Failure => "✗".fg(tailwind::ROSE.c500),
        state::Status::Running => entry.spinner.current().fg(tailwind::YELLOW.c400),
        state::Status::Waiting => Span::raw("⧖"),
        state::Status::Unknown => Span::raw("?"),
    }
}

fn render_task_table(f: &mut Frame, app: &mut App, area: Rect, tick: bool) {
    let key_legend: Title = Title::from("(Esc) quit | (↑) move up | (↓) move down")
        .alignment(Alignment::Right)
        .position(Position::Bottom);

    // TODO: useless clone
    let state_name: Title = Title::from(app.selected_state().map(|entry| entry.name.clone()).unwrap_or_default())
        .alignment(Alignment::Left)
        .position(Position::Bottom);

    let mut style_selected: Style = Style::new()
        .fg(theme::COLOR_SELECTION);

    if app.ui.selection == Selection::TaskTable {
        style_selected = style_selected.fg(theme::COLOR_SELECTION_FOCUSED);
    }

    let entries = app.task_entries.iter_mut().map(|entry| {
        if entry.status == Status::Running && tick {
            entry.spinner.next();
        }

        let row = Row::new([
            Cell::from(status(entry)),
            Cell::from(entry.time.as_str()),
            Cell::from(entry.path.as_str()),
        ]);

        row.fg(theme::COLOR_FOREGROUND).height(1)
    });

    const SELECTION_SYMBOL: &'static str = "• ";

    let border = Block::bordered()
        .border_type(BorderType::Rounded)
        .fg(theme::COLOR_BORDER)
        .title(key_legend)
        .title(state_name)
        .title_style(Style::new().fg(theme::COLOR_BLOCK_TITLE));

    let t = Table::new(entries, [
        Constraint::Length(2),
        Constraint::Length(("Tue Jul 30 03:14:39 AM".len() + 1) as u16),
        Constraint::Length(50),
    ])
    .highlight_style(style_selected)
    .highlight_symbol(SELECTION_SYMBOL)
    .highlight_spacing(HighlightSpacing::Always)
    .header(table_header())
    .block(border);

    f.render_stateful_widget(t, area, &mut app.ui.task_table.state);
}

fn render_scrollbar(f: &mut Frame, scroll: &mut ScrollbarState, area: Rect) {
    const SCROLLBAR: Scrollbar = {
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
    };

    const MARGIN: Margin = Margin::new(1, 1);

    f.render_stateful_widget(SCROLLBAR, area.inner(MARGIN), scroll);
}

fn render_error(f: &mut Frame, area: Rect, e: &Option<ErrorEntry>) {
    let Some(e) = e else {
        return
    };

    match e.severity {
        Severity::High => render_error_block(f, area, e.message.as_str()),
        Severity::Low => render_error_notice(f, area, e.message.as_str()),
    }
}

fn render_error_block(f: &mut Frame, area: Rect, message: &str) {
    let title: Title = Title::from("Error")
        .alignment(Alignment::Center)
        .position(Position::Top);

    let border = Block::bordered()
        .border_type(BorderType::Rounded)
        .fg(theme::COLOR_BORDER)
        .title(title)
        .title_style(Style::new().fg(theme::COLOR_BLOCK_TITLE));

    let p = Paragraph::new(message)
        .block(border)
        .centered()
        .wrap(Wrap { trim: false })
        .fg(theme::COLOR_ERROR);


    let area = center(
        area,
        Constraint::Percentage(50),
        Constraint::Length(10),
    );

    f.render_widget(p, area);
}

fn render_error_notice(f: &mut Frame, area: Rect, message: &str) {
}

// Thanks! ♥ https://ratatui.rs/recipes/layout/center-a-rect/
fn center(area: Rect, horizontal: Constraint, vertical: Constraint) -> Rect {
    let [area] = Layout::horizontal([horizontal])
        .flex(Flex::Center)
        .areas(area);

    let [area] = Layout::vertical([vertical]).flex(Flex::Center).areas(area);

    area
}

pub struct UI {
    pub state_list: StateList,
    pub task_table: TaskTable,

    pub selection:  Selection,
}

impl UI {
    pub fn next(&mut self) -> Action {
        match self.selection {
            Selection::StateList => match self.state_list.selected() != self.state_list.next()  {
                true  => {
                    self.task_table.deselect();

                    Action::RefreshTasks
                },
                false => Action::Tick,
            },
            Selection::TaskTable => {
                self.task_table.next();

                Action::Tick
            },
        }
    }

    pub fn previous(&mut self) -> Action {
        match self.selection {
            Selection::StateList => match self.state_list.selected() != self.state_list.previous() {
                true  => {
                    self.task_table.deselect();

                    Action::RefreshTasks
                },
                false => Action::Tick,
            },
            Selection::TaskTable => {
                self.task_table.previous();

                Action::Tick
            },
        }
    }

    pub fn switch(&mut self) -> Action {
        match self.selection {
            Selection::StateList => self.focus_task_table(),
            Selection::TaskTable => self.focus_state_list(),
        }
    }

    pub fn focus_state_list(&mut self) -> Action {
        if self.selection != Selection::StateList {
            self.selection = Selection::StateList;
        }

        Action::Tick
    }

    pub fn focus_task_table(&mut self) -> Action {
        if self.selection != Selection::TaskTable {
            self.selection = Selection::TaskTable;

            if self.task_table.state.selected().is_none() {
                self.task_table.next();
            }
        }
        
        Action::Tick
    }
}

#[derive(PartialEq)]
pub enum Selection {
    StateList,
    TaskTable,
}

pub struct TaskTable {
    state:   TableState,
    len:     usize,
    scroll:  ScrollbarState,
}

impl TaskTable {
    pub fn new(len: usize) -> Self {
        TaskTable {
            state:  TableState::default(),
            len,
            scroll: ScrollbarState::new(0),
        }
    }

    #[inline]
    pub fn deselect(&mut self) {
        self.state.select(None);
    }

    fn next(&mut self) -> usize {
        if self.len == 0 { return 0 }

        let i = match self.state.selected().filter(|&i| i < self.len.saturating_sub(1)) {
            Some(i) => i + 1,
            None => 0,
        };

        self.state.select(Some(i));
        self.scroll = self.scroll.position(i);

        return i
    }

    fn previous(&mut self) -> usize {
        if self.len == 0 { return 0 }

        let i = self.state.selected().map_or(0, |i| match i == 0 {
            true => self.len.saturating_sub(1),
            false => i - 1,
        });

        self.state.select(Some(i));
        self.scroll = self.scroll.position(i);

        return i
    }

    pub fn set_len(&mut self, len: usize) {
        self.len = len;
    }
}

pub struct StateList {
    state: ListState,

    pub len:     usize,
    pub spinner: Spinner,
}

impl StateList {
    pub fn new(len: usize) -> Self {
        StateList {
            state:  ListState::default().with_selected(Some(0)),
            len,
            spinner: Spinner::new(spinner::ARROW),
        }
    }

    fn next(&mut self) -> usize {
        if self.len == 0 { return 0 }

        let i = match self.state.selected().filter(|&i| i < self.len.saturating_sub(1)) {
            Some(i) => i + 1,
            None => 0,
        };

        self.state.select(Some(i));

        return i
    }

    fn previous(&mut self) -> usize {
        if self.len == 0 { return 0 }

        let i = self.state.selected().map_or(0, |i| match i == 0 {
            true => self.len.saturating_sub(1),
            false => i - 1,
        });

        self.state.select(Some(i));

        return i
    }

    #[inline]
    pub fn selected(&self) -> usize {
        self.state.selected().unwrap_or(0)
    }
}
