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
