use crate::app::{App, Panel};
use crate::config::config_dir;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct SessionState {
    pub projects: Vec<ProjectSession>,
    pub active_project: usize,
    pub active_panel: String,
    pub show_detail: bool,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ProjectSession {
    pub name: String,
    pub graph_selected: usize,
    pub branch_selected: usize,
    pub scroll_x: usize,
    pub collapsed_sections: Vec<String>,
}

fn session_path() -> std::path::PathBuf {
    config_dir().join("arachne").join("session.toml")
}

pub fn save(app: &App) {
    let mut projects = Vec::new();
    for proj in &app.projects {
        let collapsed: Vec<String> = app
            .collapsed_sections
            .iter()
            .filter_map(section_key_to_string)
            .collect();
        projects.push(ProjectSession {
            name: proj.name.clone(),
            graph_selected: app.graph_selected,
            branch_selected: app.branch_selected,
            scroll_x: proj.scroll_x,
            collapsed_sections: collapsed,
        });
    }

    let panel_str = match app.active_panel {
        Panel::Branches => "branches",
        Panel::Graph => "graph",
        Panel::Detail => "detail",
    };

    let state = SessionState {
        projects,
        active_project: app.active_project,
        active_panel: panel_str.to_string(),
        show_detail: app.show_detail,
    };

    if let Ok(content) = toml::to_string_pretty(&state) {
        let path = session_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, content);
    }
}

pub fn restore(app: &mut App) {
    let path = session_path();
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return,
    };
    let state: SessionState = match toml::from_str(&content) {
        Ok(s) => s,
        Err(_) => return,
    };

    // Match active project by name
    if let Some(idx) = app
        .projects
        .iter()
        .position(|p| state.projects.get(state.active_project).map(|s| &s.name) == Some(&p.name))
    {
        app.active_project = idx;
    } else {
        app.active_project = state.active_project.min(app.projects.len().saturating_sub(1));
    }

    // Restore per-project state
    for saved in &state.projects {
        if let Some(proj) = app.projects.iter_mut().find(|p| p.name == saved.name) {
            proj.scroll_x = saved.scroll_x;
        }
    }

    // Restore view state
    if let Some(saved) = state.projects.get(state.active_project) {
        let row_count = app
            .projects
            .get(app.active_project)
            .map(|p| p.rows.len())
            .unwrap_or(0);
        app.graph_selected = if row_count > 0 {
            saved.graph_selected.min(row_count - 1)
        } else {
            0
        };
        app.branch_selected = saved.branch_selected;

        // Restore collapsed sections
        for s in &saved.collapsed_sections {
            if let Some(key) = string_to_section_key(s) {
                app.collapsed_sections.insert(key);
            }
        }
    }

    app.show_detail = state.show_detail;
    app.active_panel = match state.active_panel.as_str() {
        "branches" => Panel::Branches,
        "detail" if app.show_detail => Panel::Detail,
        _ => Panel::Graph,
    };
}

fn section_key_to_string(key: &crate::ui::branch_panel::SectionKey) -> Option<String> {
    use crate::ui::branch_panel::SectionKey;
    match key {
        SectionKey::Local(i) => Some(format!("local:{i}")),
        SectionKey::Fork(i, owner) => Some(format!("fork:{i}:{owner}")),
        SectionKey::Tags(i) => Some(format!("tags:{i}")),
        SectionKey::Authors(i) => Some(format!("authors:{i}")),
    }
}

fn string_to_section_key(s: &str) -> Option<crate::ui::branch_panel::SectionKey> {
    use crate::ui::branch_panel::SectionKey;
    let parts: Vec<&str> = s.splitn(3, ':').collect();
    match parts.as_slice() {
        ["local", idx] => idx.parse().ok().map(SectionKey::Local),
        ["fork", idx, owner] => idx
            .parse()
            .ok()
            .map(|i| SectionKey::Fork(i, owner.to_string())),
        ["tags", idx] => idx.parse().ok().map(SectionKey::Tags),
        ["authors", idx] => idx.parse().ok().map(SectionKey::Authors),
        _ => None,
    }
}
