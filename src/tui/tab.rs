pub struct TabStats<'a> {
    pub project_name: &'a str,
    pub images_count: usize,
    pub exposed_ports_count: usize,
    pub volumes_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabCommand {
    RenameProject,
    NewImage,
    EditImage,
    DeleteImage,
    AddVolume,
    DeleteVolume,
    EditEnv,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Project,
    Images,
    Volume,
    Env,
}

impl Tab {
    pub fn all() -> [Self; 4] {
        [Self::Project, Self::Images, Self::Volume, Self::Env]
    }

    pub fn title(self) -> &'static str {
        match self {
            Tab::Project => "Project",
            Tab::Images => "Images",
            Tab::Volume => "Volume",
            Tab::Env => "Env",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Tab::Project => Tab::Images,
            Tab::Images => Tab::Volume,
            Tab::Volume => Tab::Env,
            Tab::Env => Tab::Project,
        }
    }

    pub fn previous(self) -> Self {
        match self {
            Tab::Project => Tab::Env,
            Tab::Images => Tab::Project,
            Tab::Volume => Tab::Images,
            Tab::Env => Tab::Volume,
        }
    }

    pub fn keybind_hint(self) -> &'static str {
        match self {
            Tab::Project => "r rename project",
            Tab::Images => "n new image, e edit image, d delete image",
            Tab::Volume => "a add volume, d delete volume",
            Tab::Env => "e edit env",
        }
    }

    pub fn action_labels(self) -> &'static [&'static str] {
        match self {
            Tab::Project => &["R: rename project"],
            Tab::Images => &["N: new image", "E: edit image", "D: delete image"],
            Tab::Volume => &["A: add volume", "D: delete volume"],
            Tab::Env => &["E: edit env"],
        }
    }

    pub fn active_sidebar_text(self, stats: &TabStats<'_>, actions_text: &str) -> String {
        match self {
            Tab::Project => format!(
                "Directory: {}\nTemp: 74°C\nCPU: 12%\nMem: 418MB\n\nAction: {}",
                stats.project_name, actions_text
            ),
            Tab::Images => format!(
                "Loaded images: {}\nExposed ports: {}\n\nAction: {}",
                stats.images_count, stats.exposed_ports_count, actions_text
            ),
            Tab::Volume => format!("Volumes: {}\n\nAction: {}", stats.volumes_count, actions_text),
            Tab::Env => format!("Environment settings\nplaceholder\n\nAction: {}", actions_text),
        }
    }

    pub fn inactive_summary(self, stats: &TabStats<'_>) -> String {
        match self {
            Tab::Project => "Compose preview".to_string(),
            Tab::Images => format!("{} images", stats.images_count),
            Tab::Volume => format!("{} volumes", stats.volumes_count),
            Tab::Env => "Env vars".to_string(),
        }
    }

    pub fn command_for_key(self, key: char) -> Option<TabCommand> {
        match (self, key) {
            (Tab::Project, 'r') => Some(TabCommand::RenameProject),
            (Tab::Images, 'n') => Some(TabCommand::NewImage),
            (Tab::Images, 'e') => Some(TabCommand::EditImage),
            (Tab::Images, 'd') => Some(TabCommand::DeleteImage),
            (Tab::Volume, 'a') => Some(TabCommand::AddVolume),
            (Tab::Volume, 'd') => Some(TabCommand::DeleteVolume),
            (Tab::Env, 'e') => Some(TabCommand::EditEnv),
            _ => None,
        }
    }

    pub fn keybind_action(self, key: char) -> Option<&'static str> {
        match (self, key) {
            (Tab::Project, 'r') => Some("rename project requested"),
            (Tab::Images, 'n') => Some("new image requested"),
            (Tab::Volume, 'a') => Some("add volume requested"),
            (Tab::Env, 'e') => Some("edit env requested"),
            _ => None,
        }
    }
}
