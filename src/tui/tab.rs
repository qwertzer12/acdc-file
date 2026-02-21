#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Project,
    Images,
    Env,
    Network,
}

impl Tab {
    pub fn all() -> [Self; 4] {
        [Self::Project, Self::Images, Self::Env, Self::Network]
    }

    pub fn title(self) -> &'static str {
        match self {
            Tab::Project => "Project",
            Tab::Images => "Images",
            Tab::Env => "Env",
            Tab::Network => "Network",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Tab::Project => Tab::Images,
            Tab::Images => Tab::Env,
            Tab::Env => Tab::Network,
            Tab::Network => Tab::Project,
        }
    }

    pub fn previous(self) -> Self {
        match self {
            Tab::Project => Tab::Network,
            Tab::Images => Tab::Project,
            Tab::Env => Tab::Images,
            Tab::Network => Tab::Env,
        }
    }

    pub fn keybind_hint(self) -> &'static str {
        match self {
            Tab::Project => "r rename project",
            Tab::Images => "n new image, e edit image, d delete image",
            Tab::Env => "e edit env",
            Tab::Network => "w edit network",
        }
    }

    pub fn keybind_action(self, key: char) -> Option<&'static str> {
        match (self, key) {
            (Tab::Project, 'r') => Some("rename project requested"),
            (Tab::Images, 'n') => Some("new image requested"),
            (Tab::Env, 'e') => Some("edit env requested"),
            (Tab::Network, 'w') => Some("edit network requested"),
            _ => None,
        }
    }
}
