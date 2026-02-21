use crossterm::event::KeyCode;

use crate::api;
use crate::tui::{
    app::{
        App, ConfigureField, EnvInputField, EnvVar, FocusArea, ImageEntry, ModalState,
        MountExistingField, MountInputField, VolumeEntry, VolumeMount,
    },
    tab::{Tab, TabCommand},
};

pub enum LoopControl {
    Continue,
    Exit,
}

fn default_service_name(repo: &str, current_len: usize) -> String {
    let base: String = repo
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                ch
            } else {
                '_'
            }
        })
        .collect();

    if base.is_empty() {
        format!("service_{}", current_len + 1)
    } else {
        base
    }
}

fn preferred_container_port(ports: &[u16]) -> Option<u16> {
    let preferred = [80, 443, 8080, 3000, 5000, 5432, 3306, 6379];
    for candidate in preferred {
        if ports.contains(&candidate) {
            return Some(candidate);
        }
    }
    ports.first().copied()
}

fn suggested_port_mapping(app: &App, suggested_container_port: Option<u16>) -> String {
    let fallback = app.next_port_mapping();
    let host = fallback
        .split(':')
        .next()
        .unwrap_or("8000")
        .to_string();

    if let Some(container) = suggested_container_port {
        format!("{host}:{container}")
    } else {
        fallback
    }
}

fn split_port_mapping(mapping: &str) -> (String, String) {
    let trimmed = mapping.trim();
    if let Some((host, container)) = trimmed.split_once(':') {
        (host.trim().to_string(), container.trim().to_string())
    } else {
        (trimmed.to_string(), String::from("80"))
    }
}

fn default_mount_target() -> String {
    "/data".to_string()
}

fn default_volume_name(app: &App) -> String {
    format!("volume_{}", app.volumes.len() + 1)
}

pub fn handle_key(app: &mut App, key_code: KeyCode) -> LoopControl {
    if app.modal.is_some() {
        return handle_modal_key(app, key_code);
    }

    match key_code {
        KeyCode::Char('q') | KeyCode::Esc => LoopControl::Exit,
        KeyCode::Tab => {
            app.focus = app.focus.next();
            LoopControl::Continue
        }
        KeyCode::Left | KeyCode::Char('h') => {
            app.focus = FocusArea::Sidebar;
            LoopControl::Continue
        }
        KeyCode::Right | KeyCode::Char('l') => {
            app.focus = FocusArea::Main;
            LoopControl::Continue
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if matches!(app.focus, FocusArea::Sidebar) {
                app.active_tab = app.active_tab.previous();
            } else if matches!(app.focus, FocusArea::Main)
                && matches!(app.active_tab, Tab::Images)
                && !app.images.is_empty()
                && app.images_selected > 0
            {
                app.images_selected -= 1;
            } else if matches!(app.focus, FocusArea::Main)
                && matches!(app.active_tab, Tab::Volume)
                && !app.volumes.is_empty()
                && app.volumes_selected > 0
            {
                app.volumes_selected -= 1;
            }
            LoopControl::Continue
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if matches!(app.focus, FocusArea::Sidebar) {
                app.active_tab = app.active_tab.next();
            } else if matches!(app.focus, FocusArea::Main)
                && matches!(app.active_tab, Tab::Images)
                && !app.images.is_empty()
            {
                app.images_selected = (app.images_selected + 1).min(app.images.len() - 1);
            } else if matches!(app.focus, FocusArea::Main)
                && matches!(app.active_tab, Tab::Volume)
                && !app.volumes.is_empty()
            {
                app.volumes_selected = (app.volumes_selected + 1).min(app.volumes.len() - 1);
            }
            LoopControl::Continue
        }
        KeyCode::Char(ch) => {
            if let Some(command) = app.active_tab.command_for_key(ch) {
                match command {
                    TabCommand::NewImage => {
                        app.modal = Some(ModalState::AddImageType {
                            input: String::new(),
                        });
                        app.push_log("add image: enter image term");
                        return LoopControl::Continue;
                    }
                    TabCommand::EditImage => {
                        if matches!(app.focus, FocusArea::Main) && !app.images.is_empty() {
                            let index = app.images_selected.min(app.images.len() - 1);
                            if let Some(image) = app.images.get(index).cloned() {
                                let (host_port_input, container_port_input) =
                                    split_port_mapping(&image.port_mapping);
                                app.modal = Some(ModalState::ConfigureImagePorts {
                                    existing_index: Some(index),
                                    namespace: image.namespace,
                                    repo: image.repo,
                                    tag: image.tag,
                                    host_port_input,
                                    container_port_input,
                                    service_name_input: image.service_name,
                                    active_field: ConfigureField::HostPort,
                                    host_port_typed: false,
                                    container_port_typed: false,
                                    service_name_typed: false,
                                });
                                app.push_log("edit image: adjust ports/name");
                                return LoopControl::Continue;
                            }
                        }
                    }
                    TabCommand::AddImageEnv => {
                        if matches!(app.focus, FocusArea::Main) && !app.images.is_empty() {
                            let index = app.images_selected.min(app.images.len() - 1);
                            app.modal = Some(ModalState::AddImageEnv {
                                image_index: index,
                                key_input: String::new(),
                                value_input: String::new(),
                                active_field: EnvInputField::Key,
                            });
                            app.push_log("add env: enter variable and value");
                            return LoopControl::Continue;
                        }
                    }
                    TabCommand::RemoveImageEnv => {
                        if matches!(app.focus, FocusArea::Main) && !app.images.is_empty() {
                            let index = app.images_selected.min(app.images.len() - 1);
                            if app.images[index].env_vars.is_empty() {
                                app.push_log("selected image has no env vars");
                            } else {
                                app.modal = Some(ModalState::RemoveImageEnv {
                                    image_index: index,
                                    selected_env: 0,
                                });
                                app.push_log("remove env: pick variable and confirm");
                                return LoopControl::Continue;
                            }
                        }
                    }
                    TabCommand::DeleteImage => {
                        if matches!(app.focus, FocusArea::Main) && !app.images.is_empty() {
                            let index = app.images_selected.min(app.images.len() - 1);
                            app.modal = Some(ModalState::ConfirmDeleteImage { index });
                            app.push_log("delete image: confirm with y");
                            return LoopControl::Continue;
                        }
                    }
                    TabCommand::MountImageVolume => {
                        if matches!(app.focus, FocusArea::Main) && !app.images.is_empty() {
                            let index = app.images_selected.min(app.images.len() - 1);
                            app.modal = Some(ModalState::SelectImageVolumeSource {
                                image_index: index,
                                selected_option: 0,
                            });
                            app.push_log("mount volume: choose existing/new/local");
                            return LoopControl::Continue;
                        }
                    }
                    TabCommand::RemoveImageVolume => {
                        if matches!(app.focus, FocusArea::Main) && !app.images.is_empty() {
                            let index = app.images_selected.min(app.images.len() - 1);
                            if app.images[index].mounts.is_empty() {
                                app.push_log("selected image has no mounted volumes");
                            } else {
                                app.modal = Some(ModalState::RemoveImageMount {
                                    image_index: index,
                                    selected_mount: 0,
                                });
                                app.push_log("unmount: pick mount and confirm");
                                return LoopControl::Continue;
                            }
                        }
                    }
                    TabCommand::AddVolume => {
                        app.modal = Some(ModalState::AddVolume {
                            input: String::new(),
                        });
                        app.push_log("add volume: enter a name");
                        return LoopControl::Continue;
                    }
                    TabCommand::DeleteVolume => {
                        if matches!(app.focus, FocusArea::Main) && !app.volumes.is_empty() {
                            let index = app.volumes_selected.min(app.volumes.len() - 1);
                            let removed = app.volumes.remove(index);
                            if app.volumes.is_empty() {
                                app.volumes_selected = 0;
                            } else if app.volumes_selected >= app.volumes.len() {
                                app.volumes_selected = app.volumes.len() - 1;
                            }
                            app.push_log(format!("deleted volume {}", removed.name));
                            return LoopControl::Continue;
                        }
                    }
                    TabCommand::RenameProject | TabCommand::EditEnv => {}
                }
            }

            if ch == 'p' {
                app.modal = Some(ModalState::ConfirmWriteCompose);
                app.push_log("write compose file: confirm with y");
                return LoopControl::Continue;
            }

            if let Some(action) = app.active_tab.keybind_action(ch) {
                app.push_log(format!("[{}] {action}", app.active_tab.title()));
            }

            LoopControl::Continue
        }
        _ => LoopControl::Continue,
    }
}

fn handle_modal_key(app: &mut App, key_code: KeyCode) -> LoopControl {
    let mut next_modal = app.modal.take();
    let mut modal_transition: Option<ModalState> = None;
    let mut close_modal = false;
    let mut should_exit_after_modal = false;
    let mut deferred_logs: Vec<String> = Vec::new();

    match key_code {
        KeyCode::Esc => {
            close_modal = true;
            deferred_logs.push("modal canceled".to_string());
        }
        KeyCode::Char('q') => return LoopControl::Exit,
        _ => match next_modal.as_mut() {
            Some(modal) => match modal {
                ModalState::AddImageType { input } => match key_code {
                    KeyCode::Char(ch) => input.push(ch),
                    KeyCode::Backspace => {
                        input.pop();
                    }
                    KeyCode::Enter => {
                        let image_term = input.trim().to_string();
                        if image_term.is_empty() {
                            app.push_log("type an image name to continue");
                        } else {
                            let runtime = tokio::runtime::Builder::new_current_thread()
                                .enable_all()
                                .build();

                            match runtime {
                                Ok(runtime) => {
                                    match runtime.block_on(api::resolve_docker_hub_repository(
                                        &image_term,
                                    )) {
                                        Ok(Some(resolved)) => match runtime.block_on(
                                            api::list_docker_hub_tags(
                                                &resolved.namespace,
                                                &resolved.repo,
                                            ),
                                        ) {
                                            Ok(all_tags) => {
                                                let filtered_tags = api::filter_tags(&all_tags, "", 30);
                                                modal_transition = Some(ModalState::SelectImageTag {
                                                    image_term,
                                                    namespace: resolved.namespace,
                                                    repo: resolved.repo,
                                                    all_tags,
                                                    query: String::new(),
                                                    filtered_tags,
                                                    selected: 0,
                                                });
                                                deferred_logs.push(
                                                    "image repo resolved; pick a tag".to_string(),
                                                );
                                            }
                                            Err(error) => {
                                                deferred_logs
                                                    .push(format!("tag fetch failed: {error}"));
                                            }
                                        },
                                        Ok(None) => {
                                            deferred_logs.push("no repo match found".to_string());
                                        }
                                        Err(error) => {
                                            deferred_logs.push(format!("repo search failed: {error}"));
                                        }
                                    }
                                }
                                Err(error) => {
                                    deferred_logs.push(format!("runtime error: {error}"));
                                }
                            }
                        }
                    }
                    _ => {}
                },
                ModalState::SelectImageTag {
                    image_term,
                    namespace,
                    repo,
                    all_tags,
                    query,
                    filtered_tags,
                    selected,
                } => {
                    let mut next_step: Option<ModalState> = None;

                    match key_code {
                        KeyCode::Down | KeyCode::Char('j') => {
                            if !filtered_tags.is_empty() {
                                *selected = (*selected + 1).min(filtered_tags.len() - 1);
                            }
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if !filtered_tags.is_empty() && *selected > 0 {
                                *selected -= 1;
                            }
                        }
                        KeyCode::Enter => {
                            if let Some(tag) = filtered_tags.get(*selected).cloned() {
                                let namespace_value = namespace.clone();
                                let repo_value = repo.clone();
                                let runtime = tokio::runtime::Builder::new_current_thread()
                                    .enable_all()
                                    .build();
                                let suggested_ports = match runtime {
                                    Ok(runtime) => match runtime.block_on(
                                        api::list_docker_hub_exposed_ports(
                                            &namespace_value,
                                            &repo_value,
                                            &tag,
                                        ),
                                    ) {
                                        Ok(ports) => ports,
                                        Err(error) => {
                                            deferred_logs.push(format!(
                                                "port suggestions unavailable: {error}"
                                            ));
                                            Vec::new()
                                        }
                                    },
                                    Err(error) => {
                                        deferred_logs.push(format!(
                                            "runtime error for port suggestion: {error}"
                                        ));
                                        Vec::new()
                                    }
                                };
                                let suggested_container_port =
                                    preferred_container_port(&suggested_ports);
                                let suggested_mapping =
                                    suggested_port_mapping(&app, suggested_container_port);
                                let (host_port_input, container_port_input) =
                                    split_port_mapping(&suggested_mapping);
                                next_step = Some(ModalState::ConfigureImagePorts {
                                    existing_index: None,
                                    namespace: namespace_value,
                                    repo: repo_value,
                                    tag: tag.clone(),
                                    host_port_input,
                                    container_port_input,
                                    service_name_input: default_service_name(repo, app.images.len()),
                                    active_field: ConfigureField::HostPort,
                                    host_port_typed: false,
                                    container_port_typed: false,
                                    service_name_typed: false,
                                });
                                if let Some(port) = suggested_container_port {
                                    deferred_logs.push(format!("suggested container port {port}"));
                                }
                            }
                        }
                        KeyCode::Backspace => {
                            query.pop();
                            *filtered_tags = api::filter_tags(all_tags, query, 30);
                            *selected = 0;
                        }
                        KeyCode::Char(ch) => {
                            query.push(ch);
                            *filtered_tags = api::filter_tags(all_tags, query, 30);
                            *selected = 0;
                        }
                        _ => {}
                    }

                    if let Some(step) = next_step {
                        modal_transition = Some(step);
                        deferred_logs.push(format!(
                            "resolved {} -> {}/{}; set ports",
                            image_term, namespace, repo
                        ));
                    }
                }
                ModalState::ConfigureImagePorts {
                    existing_index,
                    namespace,
                    repo,
                    tag,
                    host_port_input,
                    container_port_input,
                    service_name_input,
                    active_field,
                    host_port_typed,
                    container_port_typed,
                    service_name_typed,
                } => {
                    let mut should_close_modal = false;
                    let mut log_line: Option<String> = None;

                    match key_code {
                        KeyCode::Enter => {
                            let fallback_mapping = if let Some(index) = existing_index {
                                app.images
                                    .get(*index)
                                    .map(|image| image.port_mapping.clone())
                                    .unwrap_or_else(|| app.next_port_mapping())
                            } else {
                                app.next_port_mapping()
                            };
                            let (fallback_host, fallback_container) =
                                split_port_mapping(&fallback_mapping);

                            let host = if host_port_input.trim().is_empty() {
                                fallback_host
                            } else {
                                host_port_input.trim().to_string()
                            };
                            let container = if container_port_input.trim().is_empty() {
                                fallback_container
                            } else {
                                container_port_input.trim().to_string()
                            };
                            let mapping = format!("{host}:{container}");

                            let service_name = if service_name_input.trim().is_empty() {
                                default_service_name(repo, app.images.len())
                            } else {
                                service_name_input.trim().to_string()
                            };

                            let image = ImageEntry {
                                service_name,
                                namespace: namespace.clone(),
                                repo: repo.clone(),
                                tag: tag.clone(),
                                port_mapping: mapping,
                                mounts: existing_index
                                    .and_then(|index| app.images.get(index))
                                    .map(|image| image.mounts.clone())
                                    .unwrap_or_default(),
                                env_vars: existing_index
                                    .and_then(|index| app.images.get(index))
                                    .map(|image| image.env_vars.clone())
                                    .unwrap_or_default(),
                            };

                            if let Some(index) = existing_index {
                                if let Some(slot) = app.images.get_mut(*index) {
                                    *slot = image;
                                    app.images_selected = *index;
                                    log_line =
                                        Some(format!("updated image {namespace}/{repo}:{tag}"));
                                }
                            } else {
                                app.images.push(image);
                                if !app.images.is_empty() {
                                    app.images_selected = app.images.len() - 1;
                                }
                                log_line = Some(format!("added image {namespace}/{repo}:{tag}"));
                            }
                            should_close_modal = true;
                        }
                        KeyCode::Backspace => match active_field {
                            ConfigureField::HostPort => {
                                host_port_input.pop();
                                *host_port_typed = true;
                            }
                            ConfigureField::ContainerPort => {
                                container_port_input.pop();
                                *container_port_typed = true;
                            }
                            ConfigureField::Name => {
                                service_name_input.pop();
                                *service_name_typed = true;
                            }
                        },
                        KeyCode::Char(ch) => match active_field {
                            ConfigureField::HostPort => {
                                if ch.is_ascii_digit() {
                                    if !*host_port_typed {
                                        host_port_input.clear();
                                        *host_port_typed = true;
                                    }
                                    host_port_input.push(ch);
                                }
                            }
                            ConfigureField::ContainerPort => {
                                if ch.is_ascii_digit() {
                                    if !*container_port_typed {
                                        container_port_input.clear();
                                        *container_port_typed = true;
                                    }
                                    container_port_input.push(ch);
                                }
                            }
                            ConfigureField::Name => {
                                if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                                    if !*service_name_typed {
                                        service_name_input.clear();
                                        *service_name_typed = true;
                                    }
                                    service_name_input.push(ch);
                                }
                            }
                        },
                        KeyCode::Tab => {
                            *active_field = active_field.next();
                        }
                        _ => {}
                    }

                    if should_close_modal {
                        close_modal = true;
                    }
                    if let Some(line) = log_line {
                        deferred_logs.push(line);
                    }
                }
                ModalState::ConfirmDeleteImage { index } => match key_code {
                    KeyCode::Char('y') | KeyCode::Enter => {
                        if *index < app.images.len() {
                            let removed = app.images.remove(*index);
                            if app.images.is_empty() {
                                app.images_selected = 0;
                            } else if app.images_selected >= app.images.len() {
                                app.images_selected = app.images.len() - 1;
                            }
                            deferred_logs.push(format!(
                                "deleted image {}/{}:{}",
                                removed.namespace, removed.repo, removed.tag
                            ));
                        }
                        close_modal = true;
                    }
                    KeyCode::Char('n') => {
                        close_modal = true;
                        deferred_logs.push("delete canceled".to_string());
                    }
                    _ => {}
                },
                ModalState::ConfirmWriteCompose => match key_code {
                    KeyCode::Char('y') | KeyCode::Enter => {
                        let compose = app.compose_yaml();
                        match std::fs::write("docker-compose.yaml", compose) {
                            Ok(_) => {
                                deferred_logs
                                    .push("wrote docker-compose.yaml from preview".to_string());
                                should_exit_after_modal = true;
                            }
                            Err(error) => deferred_logs
                                .push(format!("failed to write docker-compose.yaml: {error}")),
                        }
                        close_modal = true;
                    }
                    KeyCode::Char('n') => {
                        close_modal = true;
                        deferred_logs.push("compose write canceled".to_string());
                    }
                    _ => {}
                },
                ModalState::AddVolume { input } => match key_code {
                    KeyCode::Char(ch) => input.push(ch),
                    KeyCode::Backspace => {
                        input.pop();
                    }
                    KeyCode::Enter => {
                        let mut name = input.trim().to_string();
                        if name.is_empty() {
                            name = format!("volume_{}", app.volumes.len() + 1);
                        }

                        app.volumes.push(VolumeEntry { name: name.clone() });
                        app.volumes_selected = app.volumes.len() - 1;
                        close_modal = true;
                        deferred_logs.push(format!("added volume {name}"));
                    }
                    _ => {}
                },
                ModalState::SelectImageVolumeSource {
                    image_index,
                    selected_option,
                } => match key_code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        if *selected_option > 0 {
                            *selected_option -= 1;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        *selected_option = (*selected_option + 1).min(2);
                    }
                    KeyCode::Enter => match *selected_option {
                        0 => {
                            if app.volumes.is_empty() {
                                modal_transition = Some(ModalState::MountNewVolume {
                                    image_index: *image_index,
                                    new_volume_input: default_volume_name(&app),
                                    target_input: default_mount_target(),
                                    active_field: MountInputField::Source,
                                    new_volume_typed: false,
                                    target_typed: false,
                                });
                                deferred_logs.push(
                                    "no existing volume; creating new volume mount".to_string(),
                                );
                            } else {
                                modal_transition = Some(ModalState::MountExistingVolume {
                                    image_index: *image_index,
                                    selected_volume: 0,
                                    target_input: default_mount_target(),
                                    active_field: MountExistingField::Volume,
                                    target_typed: false,
                                });
                            }
                        }
                        1 => {
                            modal_transition = Some(ModalState::MountNewVolume {
                                image_index: *image_index,
                                new_volume_input: default_volume_name(&app),
                                target_input: default_mount_target(),
                                active_field: MountInputField::Source,
                                new_volume_typed: false,
                                target_typed: false,
                            });
                        }
                        2 => {
                            modal_transition = Some(ModalState::MountLocalPath {
                                image_index: *image_index,
                                local_path_input: "./".to_string(),
                                target_input: default_mount_target(),
                                active_field: MountInputField::Source,
                                local_path_typed: false,
                                target_typed: false,
                            });
                        }
                        _ => {}
                    },
                    _ => {}
                },
                ModalState::MountExistingVolume {
                    image_index,
                    selected_volume,
                    target_input,
                    active_field,
                    target_typed,
                } => match key_code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        if matches!(active_field, MountExistingField::Volume) && *selected_volume > 0 {
                            *selected_volume -= 1;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if matches!(active_field, MountExistingField::Volume)
                            && !app.volumes.is_empty()
                        {
                            *selected_volume = (*selected_volume + 1).min(app.volumes.len() - 1);
                        }
                    }
                    KeyCode::Backspace => {
                        if matches!(active_field, MountExistingField::Target) {
                            target_input.pop();
                            *target_typed = true;
                        }
                    }
                    KeyCode::Char(ch) => {
                        if matches!(active_field, MountExistingField::Target)
                            && (ch.is_ascii_alphanumeric() || matches!(ch, '/' | '_' | '-' | '.'))
                        {
                            if !*target_typed {
                                target_input.clear();
                                *target_typed = true;
                            }
                            target_input.push(ch);
                        }
                    }
                    KeyCode::Tab => {
                        *active_field = active_field.next();
                    }
                    KeyCode::Enter => {
                        if app.volumes.is_empty() {
                            deferred_logs
                                .push("no named volumes available; create one first".to_string());
                        } else {
                            let chosen = app
                                .volumes
                                .get(*selected_volume)
                                .map(|volume| volume.name.clone())
                                .unwrap_or_else(|| app.volumes[0].name.clone());
                            let target = if target_input.trim().is_empty() {
                                default_mount_target()
                            } else {
                                target_input.trim().to_string()
                            };

                            if !app.volumes.iter().any(|volume| volume.name == chosen) {
                                app.volumes.push(VolumeEntry {
                                    name: chosen.clone(),
                                });
                                app.volumes_selected = app.volumes.len() - 1;
                            }

                            if let Some(image) = app.images.get_mut(*image_index) {
                                image.mounts.push(VolumeMount {
                                    source: chosen.clone(),
                                    target: target.clone(),
                                });
                                deferred_logs.push(format!(
                                    "mounted volume {chosen}:{target} on {}",
                                    image.service_name
                                ));
                                close_modal = true;
                            }
                        }
                    }
                    _ => {}
                },
                ModalState::MountNewVolume {
                    image_index,
                    new_volume_input,
                    target_input,
                    active_field,
                    new_volume_typed,
                    target_typed,
                } => match key_code {
                    KeyCode::Backspace => match active_field {
                        MountInputField::Source => {
                            new_volume_input.pop();
                            *new_volume_typed = true;
                        }
                        MountInputField::Target => {
                            target_input.pop();
                            *target_typed = true;
                        }
                    },
                    KeyCode::Char(ch) => match active_field {
                        MountInputField::Source => {
                            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                                if !*new_volume_typed {
                                    new_volume_input.clear();
                                    *new_volume_typed = true;
                                }
                                new_volume_input.push(ch);
                            }
                        }
                        MountInputField::Target => {
                            if ch.is_ascii_alphanumeric() || matches!(ch, '/' | '_' | '-' | '.') {
                                if !*target_typed {
                                    target_input.clear();
                                    *target_typed = true;
                                }
                                target_input.push(ch);
                            }
                        }
                    },
                    KeyCode::Tab => {
                        *active_field = active_field.next();
                    }
                    KeyCode::Enter => {
                        let mut source = new_volume_input.trim().to_string();
                        if source.is_empty() {
                            source = default_volume_name(&app);
                        }
                        let target = if target_input.trim().is_empty() {
                            default_mount_target()
                        } else {
                            target_input.trim().to_string()
                        };

                        if !app.volumes.iter().any(|volume| volume.name == source) {
                            app.volumes.push(VolumeEntry {
                                name: source.clone(),
                            });
                            app.volumes_selected = app.volumes.len() - 1;
                        }

                        if let Some(image) = app.images.get_mut(*image_index) {
                            image.mounts.push(VolumeMount {
                                source: source.clone(),
                                target: target.clone(),
                            });
                            deferred_logs.push(format!(
                                "mounted new volume {source}:{target} on {}",
                                image.service_name
                            ));
                            close_modal = true;
                        }
                    }
                    _ => {}
                },
                ModalState::MountLocalPath {
                    image_index,
                    local_path_input,
                    target_input,
                    active_field,
                    local_path_typed,
                    target_typed,
                } => match key_code {
                    KeyCode::Backspace => match active_field {
                        MountInputField::Source => {
                            local_path_input.pop();
                            *local_path_typed = true;
                        }
                        MountInputField::Target => {
                            target_input.pop();
                            *target_typed = true;
                        }
                    },
                    KeyCode::Char(ch) => match active_field {
                        MountInputField::Source => {
                            if ch.is_ascii_alphanumeric() || matches!(ch, '/' | '_' | '-' | '.') {
                                if !*local_path_typed {
                                    local_path_input.clear();
                                    *local_path_typed = true;
                                }
                                local_path_input.push(ch);
                            }
                        }
                        MountInputField::Target => {
                            if ch.is_ascii_alphanumeric() || matches!(ch, '/' | '_' | '-' | '.') {
                                if !*target_typed {
                                    target_input.clear();
                                    *target_typed = true;
                                }
                                target_input.push(ch);
                            }
                        }
                    },
                    KeyCode::Tab => {
                        *active_field = active_field.next();
                    }
                    KeyCode::Enter => {
                        let source = local_path_input.trim().to_string();
                        if !(source.starts_with("./") || source.starts_with('/')) {
                            deferred_logs.push("local path must start with ./ or /".to_string());
                        } else {
                            let target = if target_input.trim().is_empty() {
                                default_mount_target()
                            } else {
                                target_input.trim().to_string()
                            };

                            if let Some(image) = app.images.get_mut(*image_index) {
                                image.mounts.push(VolumeMount {
                                    source: source.clone(),
                                    target: target.clone(),
                                });
                                deferred_logs.push(format!(
                                    "mounted local path {source}:{target} on {}",
                                    image.service_name
                                ));
                                close_modal = true;
                            }
                        }
                    }
                    _ => {}
                },
                ModalState::RemoveImageMount {
                    image_index,
                    selected_mount,
                } => match key_code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        if *selected_mount > 0 {
                            *selected_mount -= 1;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if let Some(image) = app.images.get(*image_index) {
                            if !image.mounts.is_empty() {
                                *selected_mount = (*selected_mount + 1).min(image.mounts.len() - 1);
                            }
                        }
                    }
                    KeyCode::Enter | KeyCode::Char('y') => {
                        if let Some(image) = app.images.get_mut(*image_index) {
                            if !image.mounts.is_empty() {
                                let index = (*selected_mount).min(image.mounts.len().saturating_sub(1));
                                let removed = image.mounts.remove(index);
                                if !image.mounts.is_empty() && *selected_mount >= image.mounts.len() {
                                    *selected_mount = image.mounts.len() - 1;
                                }
                                deferred_logs.push(format!(
                                    "removed mount {}:{} from {}",
                                    removed.source, removed.target, image.service_name
                                ));
                            } else {
                                deferred_logs.push("selected image has no mounts".to_string());
                            }
                        }
                        close_modal = true;
                    }
                    KeyCode::Char('n') => {
                        close_modal = true;
                        deferred_logs.push("unmount canceled".to_string());
                    }
                    _ => {}
                },
                ModalState::AddImageEnv {
                    image_index,
                    key_input,
                    value_input,
                    active_field,
                } => match key_code {
                    KeyCode::Backspace => match active_field {
                        EnvInputField::Key => {
                            key_input.pop();
                        }
                        EnvInputField::Value => {
                            value_input.pop();
                        }
                    },
                    KeyCode::Char(ch) => match active_field {
                        EnvInputField::Key => {
                            if ch.is_ascii_alphanumeric() || ch == '_' {
                                key_input.push(ch.to_ascii_uppercase());
                            }
                        }
                        EnvInputField::Value => {
                            value_input.push(ch);
                        }
                    },
                    KeyCode::Tab => {
                        *active_field = active_field.next();
                    }
                    KeyCode::Enter => {
                        let key = key_input.trim().to_ascii_uppercase();
                        if key.is_empty() {
                            deferred_logs.push("env variable name is required".to_string());
                        } else if let Some(image) = app.images.get_mut(*image_index) {
                            let value = value_input.clone();
                            if let Some(existing) =
                                image.env_vars.iter_mut().find(|env| env.key == key)
                            {
                                existing.value = value.clone();
                                deferred_logs.push(format!(
                                    "updated env {key} on {}",
                                    image.service_name
                                ));
                            } else {
                                image.env_vars.push(EnvVar {
                                    key: key.clone(),
                                    value: value.clone(),
                                });
                                deferred_logs.push(format!(
                                    "added env {key} on {}",
                                    image.service_name
                                ));
                            }
                            close_modal = true;
                        }
                    }
                    _ => {}
                },
                ModalState::RemoveImageEnv {
                    image_index,
                    selected_env,
                } => match key_code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        if *selected_env > 0 {
                            *selected_env -= 1;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if let Some(image) = app.images.get(*image_index) {
                            if !image.env_vars.is_empty() {
                                *selected_env = (*selected_env + 1).min(image.env_vars.len() - 1);
                            }
                        }
                    }
                    KeyCode::Enter | KeyCode::Char('y') => {
                        if let Some(image) = app.images.get_mut(*image_index) {
                            if !image.env_vars.is_empty() {
                                let index = (*selected_env).min(image.env_vars.len() - 1);
                                let removed = image.env_vars.remove(index);
                                if !image.env_vars.is_empty() && *selected_env >= image.env_vars.len() {
                                    *selected_env = image.env_vars.len() - 1;
                                }
                                deferred_logs.push(format!(
                                    "removed env {} from {}",
                                    removed.key, image.service_name
                                ));
                            } else {
                                deferred_logs.push("selected image has no env vars".to_string());
                            }
                        }
                        close_modal = true;
                    }
                    KeyCode::Char('n') => {
                        close_modal = true;
                        deferred_logs.push("remove env canceled".to_string());
                    }
                    _ => {}
                },
            },
            None => {}
        },
    }

    if close_modal {
        next_modal = None;
    }
    if let Some(step) = modal_transition {
        next_modal = Some(step);
    }
    for line in deferred_logs {
        app.push_log(line);
    }

    app.modal = next_modal;

    if should_exit_after_modal {
        LoopControl::Exit
    } else {
        LoopControl::Continue
    }
}
