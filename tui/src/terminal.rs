use std::error::Error;
use std::io::Stdout;
use std::ops::{Deref, DerefMut};

use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};

pub struct Terminal {
    tui: ratatui::terminal::Terminal<CrosstermBackend<Stdout>>,
}

impl Deref for Terminal {
    type Target = ratatui::terminal::Terminal<CrosstermBackend<Stdout>>;

    fn deref(&self) -> &Self::Target {
        &self.tui
    }
}

impl DerefMut for Terminal {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.tui
    }
}

impl Terminal {
    pub fn init() -> Result<Self, Box<dyn Error>> {
        enable_raw_mode()?;
        let mut stdout = std::io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let tui = ratatui::terminal::Terminal::new(backend).unwrap();

        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let _ = disable_raw_mode();
            let _ = execute!(std::io::stdout(), LeaveAlternateScreen);

            hook(info);
        }));

        Ok(Self { tui })
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.tui.backend_mut(), LeaveAlternateScreen);
        let _ = self.tui.show_cursor();
    }
}
