use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

use crate::tui::{
    app::{App, FocusArea},
    tab::Tab,
    theme::THEME,
};

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

pub fn render(frame: &mut Frame, app: &App) {
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
    let log = List::new(log_items).block(pane_block("Actions", false));
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
