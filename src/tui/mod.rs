mod app;
mod handlers;
mod tab;
mod theme;
mod ui;

use crossterm::event::{self, Event};
use ratatui::DefaultTerminal;

use app::App;
use handlers::LoopControl;

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
            match handlers::handle_key(&mut app, key.code) {
                LoopControl::Continue => {}
                LoopControl::Exit => break Ok(()),
            }
        }
    }
}
