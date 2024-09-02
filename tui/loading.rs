
pub const BUBBLES: &str = "●●••⋅⋅••●●";

render_loading_screen(f, task_panel_layout[0], &mut app.ui.spinner, tick);

pub spinner:    SpinnerState,

fn render_loading_screen(f: &mut Frame, area: Rect, state: &mut SpinnerState, tick: bool) {
    let spinner = SpinnerWidget::new(spinner::BUBBLES);

    let area = center_of(
        area,
        Constraint::Length(3),
        Constraint::Length(3),
    );

    if tick {
        spinner.next(state);
    }

    f.render_stateful_widget(spinner, area, state);
}

pub struct SpinnerWidget<S> where S: AsRef<str> {
    s: S,
    size: usize,
}

impl<S> SpinnerWidget<S> where S: AsRef<str> {
    pub fn new(s: S) -> Self {
        let size = s.as_ref().chars().next()
            .expect("there must be atleast 3 chars for SpinnerWidget to not panic")
            .len_utf8();

        Self { s, size }
    }

    pub fn next(&self, state: &mut SpinnerState) {
        state.x = (state.x + self.size) % (self.s.as_ref().len() - self.size * 2);
    }
}

pub struct SpinnerState {
    x: usize,
}

impl Default for SpinnerState {
    fn default() -> Self {
        Self { x: 0 }
    }
}

impl<S> StatefulWidget for SpinnerWidget<S> where S: AsRef<str> {
    type State = SpinnerState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        // SAFETY: SpinnerWidget::next keeps the index inbounds
        let s = unsafe { self.s.as_ref().get_unchecked(state.x..state.x + self.size * 3) };
        buf.set_string(area.x, area.y, &s, Style::new());
    }
