mod app;
mod tab;
mod theme;
mod ui;

use crossterm::event::{self, Event, KeyCode};
use ratatui::DefaultTerminal;

use app::{App, ConfigureField, FocusArea, ImageEntry, ModalState};
use crate::api;

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
            if app.modal.is_some() {
                let mut next_modal = app.modal.take();
                let mut modal_transition: Option<ModalState> = None;
                let mut close_modal = false;
                let mut should_exit_after_modal = false;
                let mut deferred_logs: Vec<String> = Vec::new();

                match key.code {
                    KeyCode::Esc => {
                        close_modal = true;
                        deferred_logs.push("add image canceled".to_string());
                    }
                    KeyCode::Char('q') => break Ok(()),
                    _ => match next_modal.as_mut() {
                        Some(modal) => match modal {
                        ModalState::AddImageType { input } => {
                            match key.code {
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
                                                match runtime.block_on(api::resolve_docker_hub_repository(&image_term)) {
                                                    Ok(Some(resolved)) => {
                                                        match runtime.block_on(api::list_docker_hub_tags(&resolved.namespace, &resolved.repo)) {
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
                                                                deferred_logs.push("image repo resolved; pick a tag".to_string());
                                                            }
                                                            Err(error) => {
                                                                deferred_logs.push(format!("tag fetch failed: {error}"));
                                                            }
                                                        }
                                                    }
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
                            }
                        }
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

                            match key.code {
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
                                        next_step = Some(ModalState::ConfigureImagePorts {
                                            existing_index: None,
                                            namespace: namespace_value,
                                            repo: repo_value,
                                            tag: tag.clone(),
                                            port_input: app.next_port_mapping(),
                                            service_name_input: default_service_name(repo, app.images.len()),
                                            active_field: ConfigureField::Port,
                                        });
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
                            port_input,
                            service_name_input,
                            active_field,
                        } => {
                            let mut should_close_modal = false;
                            let mut log_line: Option<String> = None;

                            match key.code {
                                KeyCode::Enter => {
                                    let mapping = if port_input.trim().is_empty() {
                                        if let Some(index) = existing_index {
                                            app.images
                                                .get(*index)
                                                .map(|image| image.port_mapping.clone())
                                                .unwrap_or_else(|| app.next_port_mapping())
                                        } else {
                                            app.next_port_mapping()
                                        }
                                    } else {
                                        port_input.trim().to_string()
                                    };

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
                                    };

                                    if let Some(index) = existing_index {
                                        if let Some(slot) = app.images.get_mut(*index) {
                                            *slot = image;
                                            app.images_selected = *index;
                                            log_line = Some(format!(
                                                "updated image {namespace}/{repo}:{tag}"
                                            ));
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
                                KeyCode::Backspace => {
                                    match active_field {
                                        ConfigureField::Port => {
                                            port_input.pop();
                                        }
                                        ConfigureField::Name => {
                                            service_name_input.pop();
                                        }
                                    }
                                }
                                KeyCode::Char(ch) => {
                                    match active_field {
                                        ConfigureField::Port => {
                                            if ch.is_ascii_digit() || ch == ':' {
                                                port_input.push(ch);
                                            }
                                        }
                                        ConfigureField::Name => {
                                            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                                                service_name_input.push(ch);
                                            }
                                        }
                                    }
                                }
                                KeyCode::Tab => {
                                    *active_field = match active_field {
                                        ConfigureField::Port => ConfigureField::Name,
                                        ConfigureField::Name => ConfigureField::Port,
                                    };
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
                        ModalState::ConfirmDeleteImage { index } => {
                            match key.code {
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
                            }
                        }
                        ModalState::ConfirmWriteCompose => {
                            match key.code {
                                KeyCode::Char('y') | KeyCode::Enter => {
                                    let compose = app.compose_yaml();
                                    match std::fs::write("docker-compose.yaml", compose) {
                                        Ok(_) => {
                                            deferred_logs.push(
                                                "wrote docker-compose.yaml from preview".to_string(),
                                            );
                                            should_exit_after_modal = true;
                                        }
                                        Err(error) => deferred_logs.push(format!(
                                            "failed to write docker-compose.yaml: {error}"
                                        )),
                                    }
                                    close_modal = true;
                                }
                                KeyCode::Char('n') => {
                                    close_modal = true;
                                    deferred_logs.push("compose write canceled".to_string());
                                }
                                _ => {}
                            }
                        }
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
                    break Ok(());
                }

                continue;
            }

            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => break Ok(()),
                KeyCode::Tab => app.focus = app.focus.next(),
                KeyCode::Left | KeyCode::Char('h') => app.focus = FocusArea::Sidebar,
                KeyCode::Right | KeyCode::Char('l') => app.focus = FocusArea::Main,
                KeyCode::Up | KeyCode::Char('k') => {
                    if matches!(app.focus, FocusArea::Sidebar) {
                        app.active_tab = app.active_tab.previous();
                    } else if matches!(app.focus, FocusArea::Main)
                        && matches!(app.active_tab, tab::Tab::Images)
                        && !app.images.is_empty()
                        && app.images_selected > 0
                    {
                        app.images_selected -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if matches!(app.focus, FocusArea::Sidebar) {
                        app.active_tab = app.active_tab.next();
                    } else if matches!(app.focus, FocusArea::Main)
                        && matches!(app.active_tab, tab::Tab::Images)
                        && !app.images.is_empty()
                    {
                        app.images_selected = (app.images_selected + 1).min(app.images.len() - 1);
                    }
                }
                KeyCode::Char(ch) => {
                    if matches!(app.active_tab, tab::Tab::Images) && ch == 'n' {
                        app.modal = Some(ModalState::AddImageType {
                            input: String::new(),
                        });
                        app.push_log("add image: enter image term");
                        continue;
                    }

                    if matches!(app.active_tab, tab::Tab::Images)
                        && matches!(app.focus, FocusArea::Main)
                        && !app.images.is_empty()
                        && ch == 'e'
                    {
                        let index = app.images_selected.min(app.images.len() - 1);
                        if let Some(image) = app.images.get(index).cloned() {
                            app.modal = Some(ModalState::ConfigureImagePorts {
                                existing_index: Some(index),
                                namespace: image.namespace,
                                repo: image.repo,
                                tag: image.tag,
                                port_input: image.port_mapping,
                                service_name_input: image.service_name,
                                active_field: ConfigureField::Port,
                            });
                            app.push_log("edit image: adjust ports/name");
                            continue;
                        }
                    }

                    if matches!(app.active_tab, tab::Tab::Images)
                        && matches!(app.focus, FocusArea::Main)
                        && !app.images.is_empty()
                        && ch == 'd'
                    {
                        let index = app.images_selected.min(app.images.len() - 1);
                        app.modal = Some(ModalState::ConfirmDeleteImage { index });
                        app.push_log("delete image: confirm with y");
                        continue;
                    }

                    if ch == 'p' {
                        app.modal = Some(ModalState::ConfirmWriteCompose);
                        app.push_log("write compose file: confirm with y");
                        continue;
                    }

                    if let Some(action) = app.active_tab.keybind_action(ch) {
                        app.push_log(format!("[{}] {action}", app.active_tab.title()));
                    }
                }
                _ => {}
            }
        }
    }
}
