use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

struct Theme {
    active_border: Color,
    inactive_border: Color,
    header_fg: Color,
    header_bg: Color,
    footer_fg: Color,
    text_fg: Color,
}

const THEME: Theme = Theme {
    active_border: Color::Blue,
    inactive_border: Color::DarkGray,
    header_fg: Color::Black,
    header_bg: Color::Blue,
    footer_fg: Color::DarkGray,
    text_fg: Color::White,
};

#[derive(Debug, Clone, Copy)]
enum FocusArea {
    Sidebar,
    Main,
}

impl FocusArea {
    fn next(self) -> Self {
        match self {
            FocusArea::Sidebar => FocusArea::Main,
            FocusArea::Main => FocusArea::Sidebar,
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[derive(PartialEq, Eq)]
enum Tab {
    Project,
    Images,
    Env,
    Network,
}

impl Tab {
    fn all() -> [Self; 4] {
        [Self::Project, Self::Images, Self::Env, Self::Network]
    }

    fn title(self) -> &'static str {
        match self {
            Tab::Project => "Project",
            Tab::Images => "Images",
            Tab::Env => "Env",
            Tab::Network => "Network",
        }
    }

    fn next(self) -> Self {
        match self {
            Tab::Project => Tab::Images,
            Tab::Images => Tab::Env,
            Tab::Env => Tab::Network,
            Tab::Network => Tab::Project,
        }
    }

    fn previous(self) -> Self {
        match self {
            Tab::Project => Tab::Network,
            Tab::Images => Tab::Project,
            Tab::Env => Tab::Images,
            Tab::Network => Tab::Env,
        }
    }

    fn keybind_hint(self) -> &'static str {
        match self {
            Tab::Project => "r rename project",
            Tab::Images => "n new image",
            Tab::Env => "e edit env",
            Tab::Network => "w edit network",
        }
    }

    fn keybind_action(self, key: char) -> Option<&'static str> {
        match (self, key) {
            (Tab::Project, 'r') => Some("rename project requested"),
            (Tab::Images, 'n') => Some("new image requested"),
            (Tab::Env, 'e') => Some("edit env requested"),
            (Tab::Network, 'w') => Some("edit network requested"),
            _ => None,
        }
    }
}

struct App {
    focus: FocusArea,
    active_tab: Tab,
    project_name: String,
    command_log: Vec<String>,
}

impl App {
    fn new() -> Self {
        let project_name = std::env::current_dir()
            .ok()
            .and_then(|path| path.file_name().map(|name| name.to_string_lossy().to_string()))
            .unwrap_or_else(|| "unknown-project".to_string());

        Self {
            focus: FocusArea::Sidebar,
            active_tab: Tab::Project,
            project_name,
            command_log: vec!["ready".to_string()],
        }
    }

    fn push_log(&mut self, line: impl Into<String>) {
        self.command_log.push(line.into());
        if self.command_log.len() > 5 {
            self.command_log.remove(0);
        }
    }
}

pub fn run() -> color_eyre::Result<()> {
    color_eyre::install()?;
    ratatui::run(app)?;
    Ok(())
}

fn app(terminal: &mut DefaultTerminal) -> std::io::Result<()> {
    let mut app: App = App::new();

    loop {
        terminal.draw(|frame| render(frame, &app))?;

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

fn pane_block(title: &str, active: bool) -> Block<'_> {
    let border_style = if active {
        Style::default()
            .fg(THEME.active_border)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(THEME.inactive_border)
    };

    Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style)
}

fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();
    frame.render_widget(Clear, area);

    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(area);

    let header = Paragraph::new("acdc - Docker Compose   |   <Tab> cycle panes   q quit")
        .style(Style::default().fg(THEME.header_fg).bg(THEME.header_bg))
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(header, root[0]);

    let sidebar_width = if matches!(app.focus, FocusArea::Sidebar) {
        Constraint::Percentage(34)
    } else {
        Constraint::Percentage(22)
    };

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([sidebar_width, Constraint::Min(60)])
        .split(root[1]);

    let tabs = Tab::all();
    let sidebar_constraints: Vec<Constraint> = tabs
        .iter()
        .map(|tab| {
            if *tab == app.active_tab {
                Constraint::Min(8)
            } else {
                Constraint::Length(3)
            }
        })
        .collect();

    let sidebar = Layout::default()
        .direction(Direction::Vertical)
        .constraints(sidebar_constraints)
        .split(body[0]);

    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(12), Constraint::Length(8)])
        .split(body[1]);

    for (index, tab) in tabs.iter().enumerate() {
        let is_active = *tab == app.active_tab;
        let title = if is_active {
            format!("▶ {}", tab.title())
        } else {
            format!("  {}", tab.title())
        };

        let tab_text = if is_active {
            match tab {
                Tab::Project => format!(
                    "Directory: {}\nTemp: 74°C\nCPU: 12%\nMem: 418MB\n\nAction: {}",
                    app.project_name,
                    tab.keybind_hint()
                ),
                Tab::Images => {
                    format!("Loaded images: 3\nExposed ports: 3\n\nAction: {}", tab.keybind_hint())
                }
                Tab::Env => {
                    format!("Environment settings\nplaceholder\n\nAction: {}", tab.keybind_hint())
                }
                Tab::Network => {
                    format!("Network settings\nplaceholder\n\nAction: {}", tab.keybind_hint())
                }
            }
        } else {
            format!("Action: {}", tab.keybind_hint())
        };

        let style = if is_active {
            Style::default().add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let tab_panel = Paragraph::new(tab_text)
            .style(style)
            .block(pane_block(&title, is_active && matches!(app.focus, FocusArea::Sidebar)));
        frame.render_widget(tab_panel, sidebar[index]);
    }

    let main_text = match app.active_tab {
        Tab::Project => {
            "version: \"3.9\"\nservices:\n  web:\n    image: nginx:latest\n    ports:\n      - \"8080:80\"\n  db:\n    image: postgres:16\n    ports:\n      - \"5432:5432\"\n"
        }
        Tab::Images => {
            "nginx:latest         -> 8080:80\npostgres:16          -> 5432:5432\nredis:7-alpine       -> 6379:6379"
        }
        Tab::Env => "Env tab placeholder\n\nUse this panel for environment variables and profile toggles.",
        Tab::Network => "Network tab placeholder\n\nUse this panel for network names, drivers, and aliases.",
    };

    let main_panel = Paragraph::new(main_text)
        .style(Style::default().fg(THEME.text_fg))
        .block(pane_block(app.active_tab.title(), matches!(app.focus, FocusArea::Main)));
    frame.render_widget(main_panel, right[0]);

    let log_items: Vec<ListItem> = app
        .command_log
        .iter()
        .map(|entry| ListItem::new(entry.as_str()))
        .collect();
    let log = List::new(log_items)
        .block(pane_block("Actions", false));
    frame.render_widget(log, right[1]);

    let footer_text = format!(
        "focus: {:?}   tab: {}   keys: Tab switch focus, j/k tab select, {}, q quit",
        app.focus,
        app.active_tab.title(),
        app.active_tab.keybind_hint()
    );
    let footer = Paragraph::new(footer_text)
        .style(Style::default().fg(THEME.footer_fg))
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(footer, root[2]);
}
