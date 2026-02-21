use crate::tui::tab::Tab;

#[derive(Debug, Clone, Copy)]
pub enum FocusArea {
    Sidebar,
    Main,
}

impl FocusArea {
    pub fn next(self) -> Self {
        match self {
            FocusArea::Sidebar => FocusArea::Main,
            FocusArea::Main => FocusArea::Sidebar,
        }
    }
}

pub struct App {
    pub focus: FocusArea,
    pub active_tab: Tab,
    pub project_name: String,
    pub command_log: Vec<String>,
}

impl App {
    pub fn new() -> Self {
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

    pub fn push_log(&mut self, line: impl Into<String>) {
        self.command_log.push(line.into());
        if self.command_log.len() > 5 {
            self.command_log.remove(0);
        }
    }
}
