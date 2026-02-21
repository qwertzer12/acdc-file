use crate::tui::tab::Tab;

#[derive(Debug, Clone)]
pub struct ImageEntry {
    pub service_name: String,
    pub namespace: String,
    pub repo: String,
    pub tag: String,
    pub port_mapping: String,
    pub mounts: Vec<VolumeMount>,
    pub env_vars: Vec<EnvVar>,
}

#[derive(Debug, Clone)]
pub struct VolumeMount {
    pub source: String,
    pub target: String,
}

#[derive(Debug, Clone)]
pub struct EnvVar {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct VolumeEntry {
    pub name: String,
}

#[derive(Debug, Clone, Copy)]
pub enum ConfigureField {
    HostPort,
    ContainerPort,
    Name,
}

impl ConfigureField {
    pub fn next(self) -> Self {
        match self {
            ConfigureField::HostPort => ConfigureField::ContainerPort,
            ConfigureField::ContainerPort => ConfigureField::Name,
            ConfigureField::Name => ConfigureField::HostPort,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ModalState {
    AddImageType {
        input: String,
    },
    SelectImageTag {
        image_term: String,
        namespace: String,
        repo: String,
        all_tags: Vec<String>,
        query: String,
        filtered_tags: Vec<String>,
        selected: usize,
    },
    ConfigureImagePorts {
        existing_index: Option<usize>,
        namespace: String,
        repo: String,
        tag: String,
        host_port_input: String,
        container_port_input: String,
        service_name_input: String,
        active_field: ConfigureField,
        host_port_typed: bool,
        container_port_typed: bool,
        service_name_typed: bool,
    },
    ConfirmDeleteImage {
        index: usize,
    },
    ConfirmWriteCompose,
    AddVolume {
        input: String,
    },
    SelectImageVolumeSource {
        image_index: usize,
        selected_option: usize,
    },
    MountExistingVolume {
        image_index: usize,
        selected_volume: usize,
        target_input: String,
        active_field: MountExistingField,
        target_typed: bool,
    },
    MountNewVolume {
        image_index: usize,
        new_volume_input: String,
        target_input: String,
        active_field: MountInputField,
        new_volume_typed: bool,
        target_typed: bool,
    },
    MountLocalPath {
        image_index: usize,
        local_path_input: String,
        target_input: String,
        active_field: MountInputField,
        local_path_typed: bool,
        target_typed: bool,
    },
    RemoveImageMount {
        image_index: usize,
        selected_mount: usize,
    },
    AddImageEnv {
        image_index: usize,
        key_input: String,
        value_input: String,
        active_field: EnvInputField,
    },
    RemoveImageEnv {
        image_index: usize,
        selected_env: usize,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum EnvInputField {
    Key,
    Value,
}

impl EnvInputField {
    pub fn next(self) -> Self {
        match self {
            EnvInputField::Key => EnvInputField::Value,
            EnvInputField::Value => EnvInputField::Key,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MountExistingField {
    Volume,
    Target,
}

impl MountExistingField {
    pub fn next(self) -> Self {
        match self {
            MountExistingField::Volume => MountExistingField::Target,
            MountExistingField::Target => MountExistingField::Volume,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MountInputField {
    Source,
    Target,
}

impl MountInputField {
    pub fn next(self) -> Self {
        match self {
            MountInputField::Source => MountInputField::Target,
            MountInputField::Target => MountInputField::Source,
        }
    }
}

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
    pub images: Vec<ImageEntry>,
    pub images_selected: usize,
    pub volumes: Vec<VolumeEntry>,
    pub volumes_selected: usize,
    pub modal: Option<ModalState>,
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
            images: Vec::new(),
            images_selected: 0,
            volumes: Vec::new(),
            volumes_selected: 0,
            modal: None,
        }
    }

    pub fn push_log(&mut self, line: impl Into<String>) {
        self.command_log.push(line.into());
        if self.command_log.len() > 5 {
            self.command_log.remove(0);
        }
    }

    pub fn next_port_mapping(&self) -> String {
        let host_port = 8000 + self.images.len() as u16;
        format!("{host_port}:80")
    }

    pub fn total_exposed_ports(&self) -> usize {
        self.images
            .iter()
            .filter(|image| image.port_mapping.contains(':'))
            .count()
    }

    pub fn compose_yaml(&self) -> String {
        let mut output = String::from("services:\n");

        if self.images.is_empty() {
            output.push_str("  # No services yet\n");
            output.push_str("  # Press n in Images tab to add one\n");
            return output;
        }

        for image in &self.images {
            let image_ref = if image.namespace == "library" {
                format!("{}:{}", image.repo, image.tag)
            } else {
                format!("{}/{}:{}", image.namespace, image.repo, image.tag)
            };

            output.push_str(&format!(
                "  {}:\n    image: {}\n    ports:\n      - \"{}\"\n",
                image.service_name, image_ref, image.port_mapping
            ));

            if !image.mounts.is_empty() {
                output.push_str("    volumes:\n");
                for mount in &image.mounts {
                    output.push_str(&format!(
                        "      - \"{}:{}\"\n",
                        mount.source, mount.target
                    ));
                }
            }

            if !image.env_vars.is_empty() {
                output.push_str("    environment:\n");
                for env in &image.env_vars {
                    output.push_str(&format!("      - {}={}\n", env.key, env.value));
                }
            }
        }

        if !self.volumes.is_empty() {
            output.push_str("\nvolumes:\n");
            for volume in &self.volumes {
                output.push_str(&format!("  {}:\n", volume.name));
            }
        }

        output
    }
}
