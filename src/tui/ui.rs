use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

use crate::tui::{
    app::{
        App, ConfigureField, EnvInputField, FocusArea, ModalState, MountExistingField,
        MountInputField,
    },
    tab::{Tab, TabStats},
    theme::THEME,
};

const ACTION_SPACING: &str = "\n        ";

fn actions_text(actions: &[&str]) -> String {
    actions.join(ACTION_SPACING)
}

fn visible_window(total: usize, selected: usize, view_height: usize) -> (usize, usize) {
    if total == 0 || view_height == 0 {
        return (0, 0);
    }

    if total <= view_height {
        return (0, total);
    }

    let half = view_height / 2;
    let mut start = selected.saturating_sub(half);
    if start + view_height > total {
        start = total - view_height;
    }

    (start, start + view_height)
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
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
    let tab_stats = TabStats {
        project_name: &app.project_name,
        images_count: app.images.len(),
        exposed_ports_count: app.total_exposed_ports(),
        volumes_count: app.volumes.len(),
    };
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
            tab.active_sidebar_text(&tab_stats, &actions_text(tab.action_labels()))
        } else {
            tab.inactive_summary(&tab_stats)
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
        Tab::Project => app.compose_yaml(),
        Tab::Images => String::new(),
        Tab::Volume => String::new(),
        Tab::Env => {
            "Env tab placeholder\n\nUse this panel for environment variables and profile toggles."
                .to_string()
        }
    };

    if matches!(app.active_tab, Tab::Images) {
        let image_items: Vec<ListItem> = if app.images.is_empty() {
            vec![
                ListItem::new("No images yet."),
                ListItem::new("Press n in Images tab to add one."),
            ]
        } else {
            let selected = app.images_selected.min(app.images.len() - 1);
            let list_height = right[0].height.saturating_sub(2) as usize;
            let (start, end) = visible_window(app.images.len(), selected, list_height.max(1));

            app.images[start..end]
                .iter()
                .enumerate()
                .map(|(offset, image)| {
                    let index = start + offset;
                    ListItem::new(format!(
                        "{} {}: {}/{}:{}   ->   {}   mounts:{}   env:{}",
                        if index == selected { "▶" } else { " " },
                        image.service_name,
                        image.namespace,
                        image.repo,
                        image.tag,
                        image.port_mapping,
                        image.mounts.len(),
                        image.env_vars.len()
                    ))
                    .style(if index == selected {
                        Style::default().add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    })
                })
                .collect()
        };

        let images_panel = List::new(image_items)
            .style(Style::default().fg(THEME.text_fg))
            .block(pane_block("Images", matches!(app.focus, FocusArea::Main)));
        frame.render_widget(images_panel, right[0]);
    } else if matches!(app.active_tab, Tab::Volume) {
        let volume_items: Vec<ListItem> = if app.volumes.is_empty() {
            vec![
                ListItem::new("No volumes yet."),
                ListItem::new("Press a in Volume tab to add one."),
            ]
        } else {
            let selected = app.volumes_selected.min(app.volumes.len() - 1);
            let list_height = right[0].height.saturating_sub(2) as usize;
            let (start, end) = visible_window(app.volumes.len(), selected, list_height.max(1));

            app.volumes[start..end]
                .iter()
                .enumerate()
                .map(|(offset, volume)| {
                    let index = start + offset;
                    ListItem::new(format!(
                        "{} {}",
                        if index == selected { "▶" } else { " " },
                        volume.name
                    ))
                    .style(if index == selected {
                        Style::default().add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    })
                })
                .collect()
        };

        let volume_panel = List::new(volume_items)
            .style(Style::default().fg(THEME.text_fg))
            .block(pane_block("Volumes", matches!(app.focus, FocusArea::Main)));
        frame.render_widget(volume_panel, right[0]);
    } else {
        let main_panel = Paragraph::new(main_text)
            .style(Style::default().fg(THEME.text_fg))
            .block(pane_block(app.active_tab.title(), matches!(app.focus, FocusArea::Main)));
        frame.render_widget(main_panel, right[0]);
    }

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

    if let Some(modal) = &app.modal {
        let popup = centered_rect(70, 70, area);
        frame.render_widget(Clear, popup);

        match modal {
            ModalState::AddImageType { input } => {
                let text = format!(
                    "Add New Image\n\nType image name/org (examples: python, nginx, node)\n\nImage: {input}\n\nEnter: resolve and fetch tags\nEsc: cancel"
                );
                let widget = Paragraph::new(text)
                    .alignment(Alignment::Left)
                    .block(pane_block("New Image", true));
                frame.render_widget(widget, popup);
            }
            ModalState::SelectImageTag {
                image_term,
                namespace,
                repo,
                query,
                filtered_tags,
                selected,
                ..
            } => {
                let sections = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(6),
                        Constraint::Min(8),
                        Constraint::Length(3),
                    ])
                    .split(popup);

                let header_text = format!(
                    "Resolved image term: {image_term}\nUsing repo: {}/{}\nFilter tags: {}",
                    namespace, repo, query
                );
                let header = Paragraph::new(header_text).block(pane_block("Select Tag", true));
                frame.render_widget(header, sections[0]);

                let tag_items: Vec<ListItem> = if filtered_tags.is_empty() {
                    vec![ListItem::new("No tags match this query.")]
                } else {
                    let selected = (*selected).min(filtered_tags.len() - 1);
                    let view_height = sections[1].height.saturating_sub(2) as usize;
                    let (start, end) = visible_window(filtered_tags.len(), selected, view_height.max(1));

                    filtered_tags[start..end]
                        .iter()
                        .enumerate()
                        .map(|(offset, tag)| {
                            let index = start + offset;
                            if index == selected {
                                ListItem::new(format!("▶ {tag}"))
                                    .style(Style::default().add_modifier(Modifier::BOLD))
                            } else {
                                ListItem::new(format!("  {tag}"))
                            }
                        })
                        .collect()
                };
                let tags = List::new(tag_items)
                    .style(Style::default().fg(THEME.text_fg))
                    .block(pane_block("Tags", true));
                frame.render_widget(tags, sections[1]);

                let hint = Paragraph::new("Type to fuzzy filter  |  j/k or arrows to move  |  Enter add image  |  Esc cancel")
                    .alignment(Alignment::Left)
                    .block(Block::default().borders(Borders::ALL));
                frame.render_widget(hint, sections[2]);
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
                host_port_typed: _,
                container_port_typed: _,
                service_name_typed: _,
            } => {
                let text = format!(
                    "{}\n\nImage: {}/{}:{}\n\n{} In port (host): {}\n{} Out port (container): {}\n{} Service name: {}\n\nTab: switch field  |  Enter: save  |  Esc: cancel",
                    if existing_index.is_some() {
                        "Edit Image"
                    } else {
                        "Configure Image"
                    },
                    namespace,
                    repo,
                    tag,
                    if matches!(active_field, ConfigureField::HostPort) { ">" } else { " " },
                    host_port_input,
                    if matches!(active_field, ConfigureField::ContainerPort) { ">" } else { " " },
                    container_port_input,
                    if matches!(active_field, ConfigureField::Name) { ">" } else { " " },
                    service_name_input
                );
                let widget = Paragraph::new(text)
                    .alignment(Alignment::Left)
                    .block(pane_block("Ports", true));
                frame.render_widget(widget, popup);
            }
            ModalState::ConfirmDeleteImage { index } => {
                let text = if let Some(image) = app.images.get(*index) {
                    format!(
                        "Delete Image\n\n{}: {}/{}:{}\nports: {}\n\nPress y (or Enter) to confirm\nPress n or Esc to cancel",
                        image.service_name,
                        image.namespace,
                        image.repo,
                        image.tag,
                        image.port_mapping
                    )
                } else {
                    "Delete Image\n\nSelected image not found.\nPress Esc to cancel".to_string()
                };
                let widget = Paragraph::new(text)
                    .alignment(Alignment::Left)
                    .block(pane_block("Confirm Delete", true));
                frame.render_widget(widget, popup);
            }
            ModalState::ConfirmWriteCompose => {
                let text = format!(
                    "Write Compose File\n\nThis will write ./docker-compose.yaml using the current Project preview.\n\n{}\n\nPress y (or Enter) to confirm\nPress n or Esc to cancel",
                    if app.images.is_empty() {
                        "Warning: no images are configured yet."
                    } else {
                        ""
                    }
                );
                let widget = Paragraph::new(text)
                    .alignment(Alignment::Left)
                    .block(pane_block("Confirm Write", true));
                frame.render_widget(widget, popup);
            }
            ModalState::AddVolume { input } => {
                let text = format!(
                    "Add Volume\n\nEnter volume name:\n\nName: {input}\n\nEnter: add volume\nEsc: cancel"
                );
                let widget = Paragraph::new(text)
                    .alignment(Alignment::Left)
                    .block(pane_block("New Volume", true));
                frame.render_widget(widget, popup);
            }
            ModalState::SelectImageVolumeSource {
                image_index,
                selected_option,
            } => {
                let image_desc = app
                    .images
                    .get(*image_index)
                    .map(|image| {
                        format!(
                            "{}: {}/{}:{}",
                            image.service_name, image.namespace, image.repo, image.tag
                        )
                    })
                    .unwrap_or_else(|| "selected image not found".to_string());

                let options = [
                    "Use existing named volume",
                    "Create and mount a new named volume",
                    "Use local path (./ or /usr/...)",
                ];

                let options_text = options
                    .iter()
                    .enumerate()
                    .map(|(index, label)| {
                        if index == *selected_option {
                            format!("> {label}")
                        } else {
                            format!("  {label}")
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                let text = format!(
                    "Mount Volume\n\nImage: {image_desc}\n\n{options_text}\n\nj/k or arrows: move  |  Enter: choose  |  Esc: cancel"
                );
                let widget = Paragraph::new(text)
                    .alignment(Alignment::Left)
                    .block(pane_block("Mount Source", true));
                frame.render_widget(widget, popup);
            }
            ModalState::MountExistingVolume {
                image_index,
                selected_volume,
                target_input,
                active_field,
                target_typed: _,
            } => {
                let image_desc = app
                    .images
                    .get(*image_index)
                    .map(|image| image.service_name.clone())
                    .unwrap_or_else(|| "unknown-image".to_string());

                let volume_name = app
                    .volumes
                    .get((*selected_volume).min(app.volumes.len().saturating_sub(1)))
                    .map(|volume| volume.name.clone())
                    .unwrap_or_else(|| "<no volume>".to_string());

                let text = format!(
                    "Mount Existing Volume\n\nImage: {image_desc}\n\n{} Volume: {}\n{} Container path: {}\n\nTab: switch field  |  j/k: volume select  |  Enter: mount  |  Esc: cancel",
                    if matches!(active_field, MountExistingField::Volume) {
                        ">"
                    } else {
                        " "
                    },
                    volume_name,
                    if matches!(active_field, MountExistingField::Target) {
                        ">"
                    } else {
                        " "
                    },
                    target_input
                );
                let widget = Paragraph::new(text)
                    .alignment(Alignment::Left)
                    .block(pane_block("Mount Existing", true));
                frame.render_widget(widget, popup);
            }
            ModalState::MountNewVolume {
                image_index,
                new_volume_input,
                target_input,
                active_field,
                new_volume_typed: _,
                target_typed: _,
            } => {
                let image_desc = app
                    .images
                    .get(*image_index)
                    .map(|image| image.service_name.clone())
                    .unwrap_or_else(|| "unknown-image".to_string());

                let text = format!(
                    "Mount New Volume\n\nImage: {image_desc}\n\n{} Volume name: {}\n{} Container path: {}\n\nTab: switch field  |  Enter: create + mount  |  Esc: cancel",
                    if matches!(active_field, MountInputField::Source) {
                        ">"
                    } else {
                        " "
                    },
                    new_volume_input,
                    if matches!(active_field, MountInputField::Target) {
                        ">"
                    } else {
                        " "
                    },
                    target_input
                );
                let widget = Paragraph::new(text)
                    .alignment(Alignment::Left)
                    .block(pane_block("Mount New", true));
                frame.render_widget(widget, popup);
            }
            ModalState::MountLocalPath {
                image_index,
                local_path_input,
                target_input,
                active_field,
                local_path_typed: _,
                target_typed: _,
            } => {
                let image_desc = app
                    .images
                    .get(*image_index)
                    .map(|image| image.service_name.clone())
                    .unwrap_or_else(|| "unknown-image".to_string());

                let text = format!(
                    "Mount Local Path\n\nImage: {image_desc}\n\n{} Local source path: {}\n{} Container path: {}\n\nSource must start with ./ or /\nTab: switch field  |  Enter: mount  |  Esc: cancel",
                    if matches!(active_field, MountInputField::Source) {
                        ">"
                    } else {
                        " "
                    },
                    local_path_input,
                    if matches!(active_field, MountInputField::Target) {
                        ">"
                    } else {
                        " "
                    },
                    target_input
                );
                let widget = Paragraph::new(text)
                    .alignment(Alignment::Left)
                    .block(pane_block("Mount Local", true));
                frame.render_widget(widget, popup);
            }
            ModalState::RemoveImageMount {
                image_index,
                selected_mount,
            } => {
                let image = app.images.get(*image_index);
                let image_desc = image
                    .map(|entry| entry.service_name.clone())
                    .unwrap_or_else(|| "unknown-image".to_string());

                let mounts_text = if let Some(entry) = image {
                    if entry.mounts.is_empty() {
                        "No mounted volumes on this image.".to_string()
                    } else {
                        entry
                            .mounts
                            .iter()
                            .enumerate()
                            .map(|(index, mount)| {
                                if index == *selected_mount {
                                    format!("> {}:{}", mount.source, mount.target)
                                } else {
                                    format!("  {}:{}", mount.source, mount.target)
                                }
                            })
                            .collect::<Vec<_>>()
                            .join("\n")
                    }
                } else {
                    "Selected image not found.".to_string()
                };

                let text = format!(
                    "Unmount Volume\n\nImage: {image_desc}\n\n{mounts_text}\n\nj/k or arrows: move  |  Enter/y: remove  |  n/Esc: cancel"
                );
                let widget = Paragraph::new(text)
                    .alignment(Alignment::Left)
                    .block(pane_block("Unmount", true));
                frame.render_widget(widget, popup);
            }
            ModalState::AddImageEnv {
                image_index,
                key_input,
                value_input,
                active_field,
            } => {
                let image_desc = app
                    .images
                    .get(*image_index)
                    .map(|entry| entry.service_name.clone())
                    .unwrap_or_else(|| "unknown-image".to_string());

                let text = format!(
                    "Add Env Variable\n\nImage: {image_desc}\n\n{} Variable: {}\n{} Value: {}\n\nName auto-uppercases. Value is kept exactly as typed.\nTab: switch field  |  Enter: save  |  Esc: cancel",
                    if matches!(active_field, EnvInputField::Key) {
                        ">"
                    } else {
                        " "
                    },
                    key_input,
                    if matches!(active_field, EnvInputField::Value) {
                        ">"
                    } else {
                        " "
                    },
                    value_input
                );
                let widget = Paragraph::new(text)
                    .alignment(Alignment::Left)
                    .block(pane_block("Env", true));
                frame.render_widget(widget, popup);
            }
            ModalState::RemoveImageEnv {
                image_index,
                selected_env,
            } => {
                let image = app.images.get(*image_index);
                let image_desc = image
                    .map(|entry| entry.service_name.clone())
                    .unwrap_or_else(|| "unknown-image".to_string());

                let env_text = if let Some(entry) = image {
                    if entry.env_vars.is_empty() {
                        "No env vars on this image.".to_string()
                    } else {
                        entry
                            .env_vars
                            .iter()
                            .enumerate()
                            .map(|(index, env)| {
                                if index == *selected_env {
                                    format!("> {}={}", env.key, env.value)
                                } else {
                                    format!("  {}={}", env.key, env.value)
                                }
                            })
                            .collect::<Vec<_>>()
                            .join("\n")
                    }
                } else {
                    "Selected image not found.".to_string()
                };

                let text = format!(
                    "Remove Env Variable\n\nImage: {image_desc}\n\n{env_text}\n\nj/k or arrows: move  |  Enter/y: remove  |  n/Esc: cancel"
                );
                let widget = Paragraph::new(text)
                    .alignment(Alignment::Left)
                    .block(pane_block("Remove Env", true));
                frame.render_widget(widget, popup);
            }
        }
    }
}
