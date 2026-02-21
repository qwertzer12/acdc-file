mod app;
mod tab;
mod theme;
mod ui;

use crossterm::event::{self, Event, KeyCode};
use ratatui::DefaultTerminal;

use app::{App, FocusArea};

pub fn run() -> color_eyre::Result<()> {
    color_eyre::install()?;
    ratatui::run(app)?;
    Ok(())
}

fn app(terminal: &mut DefaultTerminal) -> std::io::Result<()> {
    let mut app = App::new();

    loop {
        terminal.draw(|frame| ui::render(frame, &app))?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => break Ok(()),
                KeyCode::Tab => app.focus = app.focus.next(),
                KeyCode::Left | KeyCode::Char('h') => app.focus = FocusArea::Sidebar,
                KeyCode::Right | KeyCode::Char('l') => app.focus = FocusArea::Main,
                KeyCode::Up | KeyCode::Char('k') => {
                    if matches!(app.focus, FocusArea::Sidebar) {
                        app.active_tab = app.active_tab.previous();
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if matches!(app.focus, FocusArea::Sidebar) {
                        app.active_tab = app.active_tab.next();
                    }
                }
                KeyCode::Char(ch) => {
                    if let Some(action) = app.active_tab.keybind_action(ch) {
                        app.push_log(format!("[{}] {action}", app.active_tab.title()));
                    }
                }
                _ => {}
            }
        }
    }
}
