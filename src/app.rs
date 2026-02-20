const JUST_NOW: &str = "just now";

use crate::config::Config;
use crate::data_source::{self, LocalSource, RemoteSource, ViewMode};
use crate::error::Result;
use crate::event::{AppEvent, GitHubData};
use crate::git::{repo, types::RepoData};
use crate::graph::image_cache::ImageCache;
use crate::graph::{dag::Dag, filter::filter_by_author, layout};
use crate::project::{self, Project};
use crate::screen::{ConfigAction, ConfigScreenState, Screen};
use crate::terminal_graphics::GraphicsCapability;
use crate::ui::{
    branch_panel::{self, BranchPanel, DisplayEntry, SectionKey},
    config_screen::ConfigScreen,
    detail_panel::DetailPanel,
    graph_view::GraphView,
    header_bar::{HeaderBar, PaneInfo},
    help_panel::HelpPanel,
    input::{self, Action, FilterMode},
    status_bar::StatusBar,
    theme::{self, ThemePalette, THEME_NAMES},
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders},
    Frame,
};
use std::collections::HashSet;
use tokio::sync::mpsc;

use crate::ui::toast::{Notification, NotifyLevel};

#[derive(Debug, Clone, PartialEq)]
pub enum Panel {
    Branches,
    Graph,
    Detail,
}

pub struct App {
    pub config: Config,
    pub projects: Vec<Project>,
    pub active_project: usize,
    pub event_tx: Option<mpsc::UnboundedSender<AppEvent>>,

    pub screen: Screen,
    pub active_panel: Panel,
    pub graph_scroll_y: usize,
    pub graph_selected: usize,
    pub branch_scroll: usize,
    pub branch_selected: usize,

    pub show_detail: bool,
    pub show_help: bool,
    pub show_forks: bool,
    pub loading_remote: bool,
    pub filter_mode: FilterMode,
    pub filter_text: String,
    pub author_filter_text: String,
    pub collapsed_sections: HashSet<SectionKey>,
    pub notification: Option<Notification>,
    pub(crate) cached_entries: Vec<DisplayEntry>,

    pub graphics_cap: GraphicsCapability,
    pub palette: ThemePalette,
    pub confirm_quit: bool,
    pub should_quit: bool,
}

impl App {
    pub fn new(config: Config, graphics_cap: GraphicsCapability) -> Self {
        let palette = theme::palette_for_theme(config.theme.as_deref());
        Self {
            config,
            projects: Vec::new(),
            active_project: 0,
            event_tx: None,
            screen: Screen::Graph,
            active_panel: Panel::Graph,
            graph_scroll_y: 0,
            graph_selected: 0,
            branch_scroll: 0,
            branch_selected: 0,
            show_detail: false,
            show_help: false,
            show_forks: true,
            loading_remote: false,
            filter_mode: FilterMode::Off,
            filter_text: String::new(),
            author_filter_text: String::new(),
            collapsed_sections: HashSet::new(),
            notification: None,
            cached_entries: Vec::new(),
            graphics_cap,
            palette,
            confirm_quit: false,
            should_quit: false,
        }
    }

    fn apply_theme(&mut self) {
        self.palette = theme::palette_for_theme(self.config.theme.as_deref());
    }

    pub fn has_active_notification(&self) -> bool {
        self.notification.is_some()
    }

    fn notify(&mut self, level: NotifyLevel, msg: impl Into<String>) {
        self.notification = Some(Notification {
            message: msg.into(),
            level,
            created: std::time::Instant::now(),
        });
    }

    pub fn load_repos(&mut self) -> Result<()> {
        let entries = self.config.resolved_repos();
        for entry in &entries {
            let path = expand_tilde(&entry.path);
            let r = repo::open_repo(&path)?;
            let repo_data = repo::read_repo(&r, self.config.max_commits)?;

            let current_branch = head_branch_name(&repo_data);

            let repo_name = entry
                .name
                .clone()
                .unwrap_or_else(|| repo::detect_repo_name(&r));

            let dag = Dag::from_repo_data(&repo_data);
            let result = layout::compute_layout(&dag, &repo_data, &self.config.trunk_branches);
            let time_sorted_indices = project::build_time_sorted_indices(&result.rows);
            let cached_repo_data = Some(repo_data.clone());

            let remote_source = data_source::init_github_client(&self.config, &repo_name)
                .map(|client| RemoteSource { client });

            self.projects.push(Project {
                name: repo_name,
                local_source: Some(LocalSource { repo: r }),
                remote_source,
                active_mode: ViewMode::Local,
                repo_data,
                dag,
                rows: result.rows,
                branch_index_to_name: result.branch_index_to_name,
                trunk_count: result.trunk_count,
                max_lanes: result.max_lanes,
                current_branch,
                scroll_x: 0,
                last_sync: JUST_NOW.to_string(),
                rate_limit: None,
                time_sorted_indices,
                cached_repo_data,
                github_failures: 0,
                image_cache: ImageCache::new(),
            });
        }
        self.collapsed_sections = branch_panel::auto_collapse_defaults(&self.projects);
        self.refresh_entries();
        Ok(())
    }

    fn refresh_entries(&mut self) {
        let active_slice = match self.projects.get(self.active_project) {
            Some(proj) => std::slice::from_ref(proj),
            None => &[],
        };
        self.cached_entries = branch_panel::build_entries(
            active_slice,
            &self.filter_text,
            &self.author_filter_text,
            self.show_forks,
            &self.collapsed_sections,
        );
        if self.cached_entries.is_empty() {
            self.branch_selected = 0;
        } else {
            self.branch_selected = self.branch_selected.min(self.cached_entries.len() - 1);
        }
    }

    pub fn rebuild_graph(&mut self, project_idx: usize) {
        self.rebuild_graph_inner(project_idx, false);
        self.refresh_entries();
    }

    pub fn rebuild_graph_author_only(&mut self, project_idx: usize) {
        self.rebuild_graph_inner(project_idx, true);
        self.refresh_entries();
    }

    fn rebuild_graph_inner(&mut self, project_idx: usize, author_only: bool) {
        if let Some(proj) = self.projects.get_mut(project_idx) {
            let mut data = if author_only {
                if let Some(ref cached) = proj.cached_repo_data {
                    cached.clone()
                } else {
                    return;
                }
            } else if let Some(ref local) = proj.local_source {
                match repo::read_repo(&local.repo, self.config.max_commits) {
                    Ok(d) => {
                        proj.cached_repo_data = Some(d.clone());
                        d
                    }
                    Err(e) => {
                        self.notify(NotifyLevel::Error, format!("{e}"));
                        return;
                    }
                }
            } else {
                return;
            };

            proj.current_branch = head_branch_name(&data);

            if !self.author_filter_text.is_empty() {
                filter_by_author(&mut data, &self.author_filter_text);
            }

            proj.repo_data = data;
            proj.rebuild_layout(&self.config.trunk_branches);
            proj.last_sync = JUST_NOW.to_string();
        }
    }

    pub fn handle_github_result(
        &mut self,
        project_idx: usize,
        result: std::result::Result<GitHubData, String>,
    ) {
        if let Some(proj) = self.projects.get_mut(project_idx) {
            match result {
                Ok(data) => {
                    proj.github_failures = 0;
                    proj.rate_limit = data.rate_limit;
                    proj.repo_data.branches.extend(data.branches);
                    proj.dag.merge_remote(data.commits);
                    let result = layout::compute_layout(
                        &proj.dag,
                        &proj.repo_data,
                        &self.config.trunk_branches,
                    );
                    proj.apply_layout_result(result);
                    proj.cached_repo_data = None;
                    proj.last_sync = JUST_NOW.to_string();
                    self.notification = None;
                }
                Err(e) => {
                    proj.github_failures = proj.github_failures.saturating_add(1);
                    if proj.github_polling_enabled() {
                        self.notify(NotifyLevel::Error, e);
                    } else {
                        let name = proj.name.clone();
                        self.notify(
                            NotifyLevel::Warn,
                            format!(
                                "github polling disabled for {name} \u{2014} repeated failures"
                            ),
                        );
                    }
                }
            }
        }
        self.refresh_entries();
    }

    pub fn handle_event(&mut self, event: AppEvent) {
        if self.confirm_quit {
            if let AppEvent::Key(key) = &event {
                match key.code {
                    crossterm::event::KeyCode::Char('y') => {
                        self.should_quit = true;
                        return;
                    }
                    _ => {
                        self.confirm_quit = false;
                        return;
                    }
                }
            }
        }
        match event {
            AppEvent::Key(key) => match &mut self.screen {
                Screen::Config(ref mut state) => {
                    let action = state.handle_key(key);
                    self.handle_config_action(action);
                }
                Screen::Graph => {
                    let action = input::map_key(key, self.filter_mode);
                    self.handle_graph_action(action);
                }
            },
            AppEvent::GitHubResult {
                project_idx,
                result,
            } => {
                self.handle_github_result(project_idx, result);
            }
            AppEvent::RemoteDataResult {
                project_idx,
                result,
            } => {
                self.handle_remote_data_result(project_idx, result);
            }
            AppEvent::Resize => {
                self.graphics_cap.redetect_cell_size();
                for proj in &mut self.projects {
                    proj.image_cache.clear(proj.max_lanes);
                }
            }
            _ => {}
        }
    }

    fn handle_remote_data_result(
        &mut self,
        project_idx: usize,
        result: std::result::Result<RepoData, String>,
    ) {
        self.loading_remote = false;
        if let Some(proj) = self.projects.get_mut(project_idx) {
            match result {
                Ok(data) => {
                    proj.repo_data = data;
                    proj.rebuild_layout(&self.config.trunk_branches);
                    proj.last_sync = JUST_NOW.to_string();
                    self.graph_selected = 0;
                    self.graph_scroll_y = 0;
                    self.notification = None;
                }
                Err(e) => {
                    self.notify(NotifyLevel::Error, e);
                }
            }
        }
        self.refresh_entries();
    }

    fn handle_config_action(&mut self, action: ConfigAction) {
        match action {
            ConfigAction::Close => {
                self.apply_theme();
                self.screen = Screen::Graph;
            }
            ConfigAction::Save => {
                if let Screen::Config(ref state) = self.screen {
                    let new_config = state.draft.clone();
                    if let Err(e) = new_config.save() {
                        self.notify(NotifyLevel::Error, format!("save failed: {e}"));
                        return;
                    }
                    let was_first_launch = state.first_launch;
                    self.config = new_config;
                    self.apply_theme();
                    self.screen = Screen::Graph;
                    if was_first_launch {
                        if let Err(e) = self.load_repos() {
                            self.notify(NotifyLevel::Error, format!("load repos: {e}"));
                        }
                        if let Some(ref tx) = self.event_tx {
                            let _ = tx.send(AppEvent::ConfigSaved);
                        }
                    }
                    self.notify(NotifyLevel::Info, "config saved");
                }
            }
            ConfigAction::SelectTheme => {
                if let Screen::Config(ref mut state) = self.screen {
                    let idx = state.cursor.min(THEME_NAMES.len().saturating_sub(1));
                    let name = THEME_NAMES[idx];
                    state.draft.theme = Some(name.to_string());
                    state.dirty = true;
                    self.palette = theme::palette_for_theme(Some(name));
                }
            }
            ConfigAction::Quit => {
                self.should_quit = true;
            }
            ConfigAction::QuitConfirm => {
                self.confirm_quit = true;
            }
            ConfigAction::AddItem => {
                if let Screen::Config(ref mut state) = self.screen {
                    state.add_item();
                }
            }
            ConfigAction::RemoveItem => {
                if let Screen::Config(ref mut state) = self.screen {
                    state.remove_item();
                }
            }
            ConfigAction::None => {}
        }
    }

    fn handle_graph_action(&mut self, action: Action) {
        if self.notification.is_some()
            && !matches!(action, Action::None | Action::Quit | Action::ClosePopup)
        {
            self.notification = None;
            return;
        }
        match action {
            Action::Quit => self.should_quit = true,
            Action::ScrollDown => match self.active_panel {
                Panel::Graph | Panel::Detail => {
                    if let Some(proj) = self.projects.get(self.active_project) {
                        if self.graph_selected + 1 < proj.rows.len() {
                            self.graph_selected += 1;
                        }
                    }
                }
                Panel::Branches => {
                    if !self.cached_entries.is_empty() {
                        let mut next = self.branch_selected + 1;
                        while next < self.cached_entries.len()
                            && self.cached_entries[next].is_spacer()
                        {
                            next += 1;
                        }
                        if next < self.cached_entries.len() {
                            self.branch_selected = next;
                        }
                    }
                }
            },
            Action::ScrollUp => match self.active_panel {
                Panel::Graph | Panel::Detail => {
                    if self.graph_selected > 0 {
                        self.graph_selected = self.graph_selected.saturating_sub(1);
                    }
                }
                Panel::Branches => {
                    if !self.cached_entries.is_empty() && self.branch_selected > 0 {
                        let mut prev = (self.branch_selected - 1)
                            .min(self.cached_entries.len().saturating_sub(1));
                        while prev > 0 && self.cached_entries[prev].is_spacer() {
                            prev -= 1;
                        }
                        if !self.cached_entries[prev].is_spacer() {
                            self.branch_selected = prev;
                        }
                    }
                }
            },
            Action::ScrollLeft => {
                if let Some(proj) = self.projects.get_mut(self.active_project) {
                    proj.scroll_x = proj.scroll_x.saturating_sub(4);
                }
            }
            Action::ScrollRight => {
                if let Some(proj) = self.projects.get_mut(self.active_project) {
                    proj.scroll_x = proj.scroll_x.saturating_add(4);
                }
            }
            Action::PanelLeft => {
                self.active_panel = match self.active_panel {
                    Panel::Detail => Panel::Graph,
                    Panel::Graph => Panel::Branches,
                    Panel::Branches => Panel::Branches,
                };
            }
            Action::PanelRight => {
                self.active_panel = match self.active_panel {
                    Panel::Branches => Panel::Graph,
                    Panel::Graph if self.show_detail => Panel::Detail,
                    Panel::Graph => Panel::Graph,
                    Panel::Detail => Panel::Detail,
                };
            }
            Action::NextProject => {
                if !self.projects.is_empty() {
                    self.active_project = (self.active_project + 1) % self.projects.len();
                    self.graph_selected = 0;
                    self.graph_scroll_y = 0;
                    self.refresh_entries();
                }
            }
            Action::PrevProject => {
                if !self.projects.is_empty() {
                    self.active_project = if self.active_project == 0 {
                        self.projects.len() - 1
                    } else {
                        self.active_project - 1
                    };
                    self.graph_selected = 0;
                    self.graph_scroll_y = 0;
                    self.refresh_entries();
                }
            }
            Action::ToggleViewMode => {
                if let Some(proj) = self.projects.get_mut(self.active_project) {
                    match proj.active_mode {
                        ViewMode::Local => {
                            if proj.remote_source.is_none() {
                                self.notify(
                                    NotifyLevel::Warn,
                                    "no github token \u{2014} set github_token in config or GITHUB_TOKEN env",
                                );
                                return;
                            }
                            proj.active_mode = ViewMode::Remote;
                            if let Some(ref remote) = proj.remote_source {
                                if let Some(ref tx) = self.event_tx {
                                    let client = remote.client.clone();
                                    let tx = tx.clone();
                                    let idx = self.active_project;
                                    let max = self.config.max_commits;
                                    self.loading_remote = true;
                                    tokio::spawn(async move {
                                        let result =
                                            crate::github::remote_loader::load_remote_repo_data(
                                                &client, max,
                                            )
                                            .await;
                                        let _ = tx.send(AppEvent::RemoteDataResult {
                                            project_idx: idx,
                                            result,
                                        });
                                    });
                                }
                            }
                        }
                        ViewMode::Remote => {
                            proj.active_mode = ViewMode::Local;
                            self.rebuild_graph(self.active_project);
                        }
                    }
                }
            }
            Action::ToggleDetailPanel => {
                self.show_detail = !self.show_detail;
                if !self.show_detail && self.active_panel == Panel::Detail {
                    self.active_panel = Panel::Graph;
                }
                if let Some(proj) = self.projects.get_mut(self.active_project) {
                    proj.image_cache.clear(proj.max_lanes);
                }
            }
            Action::Select => {
                if self.active_panel == Panel::Branches {
                    self.toggle_branch_section();
                } else {
                    self.show_detail = !self.show_detail;
                }
            }
            Action::ToggleForks => {
                self.show_forks = !self.show_forks;
                self.refresh_entries();
            }
            Action::Filter => self.filter_mode = FilterMode::Branch,
            Action::AuthorFilter => self.filter_mode = FilterMode::Author,
            Action::FilterChar(c) => match self.filter_mode {
                FilterMode::Branch => {
                    self.filter_text.push(c);
                    self.refresh_entries();
                }
                FilterMode::Author => self.author_filter_text.push(c),
                FilterMode::Off => {}
            },
            Action::FilterBackspace => match self.filter_mode {
                FilterMode::Branch => {
                    self.filter_text.pop();
                    self.refresh_entries();
                }
                FilterMode::Author => {
                    self.author_filter_text.pop();
                }
                FilterMode::Off => {}
            },
            Action::FilterConfirm => {
                let was_author = self.filter_mode == FilterMode::Author;
                self.filter_mode = FilterMode::Off;
                if was_author {
                    for idx in 0..self.projects.len() {
                        self.rebuild_graph_author_only(idx);
                    }
                    self.clamp_selected();
                }
                self.refresh_entries();
            }
            Action::FilterCancel => {
                match self.filter_mode {
                    FilterMode::Branch => self.filter_text.clear(),
                    FilterMode::Author => {
                        self.author_filter_text.clear();
                        self.filter_mode = FilterMode::Off;
                        for idx in 0..self.projects.len() {
                            self.rebuild_graph_author_only(idx);
                        }
                        self.clamp_selected();
                        self.refresh_entries();
                        return;
                    }
                    FilterMode::Off => {}
                }
                self.filter_mode = FilterMode::Off;
                self.refresh_entries();
            }
            Action::Refresh => {
                for idx in 0..self.projects.len() {
                    self.rebuild_graph(idx);
                }
            }
            Action::Help => self.show_help = !self.show_help,
            Action::OpenConfig => {
                let state = ConfigScreenState::new(&self.config);
                self.screen = Screen::Config(Box::new(state));
            }
            Action::ClosePopup => {
                if self.show_help {
                    self.show_help = false;
                } else if self.show_detail {
                    self.show_detail = false;
                    if self.active_panel == Panel::Detail {
                        self.active_panel = Panel::Graph;
                    }
                }
            }
            Action::None => {}
        }
    }

    pub fn flush_kitty_if_needed(
        &mut self,
        backend: &mut impl std::io::Write,
    ) -> std::io::Result<()> {
        if !self.graphics_cap.is_kitty() {
            return Ok(());
        }
        let any_dirty = self.projects.iter_mut().any(|p| p.image_cache.take_dirty());
        if any_dirty {
            write!(
                backend,
                "{}",
                crate::kitty_protocol::delete_all_kitty_images()
            )?;
        }
        Ok(())
    }

    pub fn cleanup_kitty(&self, backend: &mut impl std::io::Write) -> std::io::Result<()> {
        if self.graphics_cap.is_kitty() {
            write!(
                backend,
                "{}",
                crate::kitty_protocol::delete_all_kitty_images()
            )?;
            backend.flush()?;
        }
        Ok(())
    }

    fn clamp_selected(&mut self) {
        if let Some(proj) = self.projects.get(self.active_project) {
            if !proj.rows.is_empty() && self.graph_selected >= proj.rows.len() {
                self.graph_selected = proj.rows.len() - 1;
            }
        }
    }

    fn toggle_branch_section(&mut self) {
        if let Some(entry) = self.cached_entries.get(self.branch_selected) {
            if let Some(key) = entry.section_key() {
                let key = key.clone();
                if self.collapsed_sections.contains(&key) {
                    self.collapsed_sections.remove(&key);
                } else {
                    self.collapsed_sections.insert(key);
                }
                self.refresh_entries();
            } else if let branch_panel::EntryKind::Author { ref name } = entry.kind {
                let name = name.clone();
                if self.author_filter_text == name {
                    self.author_filter_text.clear();
                } else {
                    self.author_filter_text = name;
                }
                for idx in 0..self.projects.len() {
                    self.rebuild_graph_author_only(idx);
                }
                self.clamp_selected();
                self.refresh_entries();
            } else if let Some(tip) = entry.tip_oid() {
                if let Some(proj) = self.projects.get(self.active_project) {
                    if let Some(idx) = proj.rows.iter().position(|r| r.meta.oid == tip) {
                        self.graph_selected = idx;
                    }
                }
            }
        }
    }

    // ── Render dispatch ─────────────────────────────────────────────

    pub fn render(&mut self, frame: &mut Frame) {
        let size = frame.area();

        let bg_style = Style::default().bg(self.palette.app_bg);
        for y in size.y..size.bottom() {
            for x in size.x..size.right() {
                frame.buffer_mut()[(x, y)].set_style(bg_style);
            }
        }

        match &self.screen {
            Screen::Graph => self.render_graph_screen(frame, size),
            Screen::Config(state) => {
                let widget = ConfigScreen {
                    state,
                    palette: &self.palette,
                };
                frame.render_widget(widget, size);
            }
        }
    }

    fn render_graph_screen(&mut self, frame: &mut Frame, size: Rect) {
        let vert = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .split(size);

        let header_area = vert[0];
        let body_area = vert[1];
        let status_area = vert[2];

        self.render_header(frame, header_area);

        let panel_w = self.branch_panel_width(size.width);
        let detail_w: u16 = if self.show_detail { 50 } else { 0 };

        let mut body_constraints = vec![
            Constraint::Length(panel_w),
            Constraint::Min(1), // graph
        ];
        if self.show_detail {
            body_constraints.push(Constraint::Length(detail_w));
        }

        let body_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(body_constraints)
            .split(body_area);

        let branch_area = body_chunks[0];
        let graph_area = body_chunks[1];

        self.render_bordered_branch_panel(frame, branch_area);
        self.render_bordered_graph_panel(frame, graph_area);

        if self.show_detail && body_chunks.len() >= 3 {
            let detail_area = body_chunks[2];
            self.render_bordered_detail_panel(frame, detail_area);
        }

        self.dismiss_stale_notifications();
        self.render_status_bar(frame, status_area);
        self.render_overlays(frame, size);
    }

    fn render_bordered_branch_panel(&mut self, frame: &mut Frame, area: Rect) {
        let is_active = self.active_panel == Panel::Branches;
        let border_color = if is_active {
            self.palette.active_panel_border
        } else {
            self.palette.inactive_panel_border
        };
        let title_color = if is_active {
            self.palette.accent
        } else {
            self.palette.panel_label
        };

        let block = Block::default()
            .title(" Branches ")
            .title_style(
                Style::default()
                    .fg(title_color)
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let visible_height = inner.height as usize;
        if visible_height > 0 {
            if self.branch_selected >= self.branch_scroll + visible_height {
                self.branch_scroll = self.branch_selected - visible_height + 1;
            }
            if self.branch_selected < self.branch_scroll {
                self.branch_scroll = self.branch_selected;
            }
        }

        let branch_panel = BranchPanel {
            entries: &self.cached_entries,
            selected: self.branch_selected,
            scroll: self.branch_scroll,
            focused: is_active,
            palette: &self.palette,
        };
        frame.render_widget(branch_panel, inner);
    }

    fn render_bordered_graph_panel(&mut self, frame: &mut Frame, area: Rect) {
        let is_active = self.active_panel == Panel::Graph;
        let border_color = if is_active {
            self.palette.active_panel_border
        } else {
            self.palette.inactive_panel_border
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let visible_height = (inner.height as usize).saturating_sub(1);
        self.ensure_scroll_bounds(visible_height);

        let highlighted: HashSet<_> = self.get_highlighted_oids(&self.cached_entries);

        let render_params = self.graphics_cap.render_params();

        if let Some(proj) = self.projects.get_mut(self.active_project) {
            let palette = match proj.active_mode {
                ViewMode::Remote => self.palette.with_remote_tint(),
                ViewMode::Local => self.palette.clone(),
            };
            let graph_view = GraphView {
                rows: &proj.rows,
                scroll_y: self.graph_scroll_y,
                scroll_x: proj.scroll_x,
                selected: self.graph_selected,
                highlighted_oids: &highlighted,
                is_active,
                trunk_count: proj.trunk_count,
                max_lanes: proj.max_lanes,
                palette: &palette,
                branch_index_to_name: &proj.branch_index_to_name,
                graphics_cap: &self.graphics_cap,
                image_cache: &mut proj.image_cache,
                render_params: render_params.as_ref(),
            };
            graph_view.render_into(inner, frame.buffer_mut());
        }
    }

    fn render_bordered_detail_panel(&self, frame: &mut Frame, area: Rect) {
        let is_active = self.active_panel == Panel::Detail;
        let border_color = if is_active {
            self.palette.active_panel_border
        } else {
            self.palette.inactive_panel_border
        };
        let title_color = if is_active {
            self.palette.accent
        } else {
            self.palette.panel_label
        };

        let block = Block::default()
            .title(" Detail ")
            .title_style(
                Style::default()
                    .fg(title_color)
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        if let Some(proj) = self.projects.get(self.active_project) {
            if let Some(row) = proj.rows.get(self.graph_selected) {
                let detail = DetailPanel {
                    meta: &row.meta,
                    focused: is_active,
                    palette: &self.palette,
                };
                frame.render_widget(detail, inner);
            }
        }
    }

    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let proj = self.projects.get(self.active_project);
        let info = proj.map(|p| PaneInfo {
            name: &p.name,
            branch: &p.current_branch,
            commit_count: p.rows.len(),
        });
        let infos: Vec<PaneInfo<'_>> = info.into_iter().collect();
        let last_sync = proj.map(|p| p.last_sync.as_str()).unwrap_or("never");
        let view_mode = proj.map(|p| &p.active_mode);
        let project_count = self.projects.len();
        let header = HeaderBar {
            panes: &infos,
            last_sync,
            author_filter: &self.author_filter_text,
            view_mode,
            project_count,
            active_project_idx: self.active_project,
            palette: &self.palette,
        };
        frame.render_widget(header, area);
    }

    fn branch_panel_width(&self, term_w: u16) -> u16 {
        let max_w = branch_panel::max_entry_width(&self.cached_entries);
        let tw = term_w as usize;
        (max_w + 4).clamp(22, (tw / 3).max(22)) as u16
    }

    fn dismiss_stale_notifications(&mut self) {
        if let Some(ref n) = self.notification {
            if n.created.elapsed() > std::time::Duration::from_secs(n.level.ttl_secs()) {
                self.notification = None;
            }
        }
    }

    fn render_status_bar(&self, frame: &mut Frame, area: Rect) {
        let active = self.projects.get(self.active_project);
        let loading_msg = if self.loading_remote {
            Some("loading remote data...")
        } else {
            None
        };
        let commit_count = active.map(|p| p.rows.len()).unwrap_or(0);
        let branch_count = active.map(|p| p.repo_data.branches.len()).unwrap_or(0);
        let status = StatusBar {
            branch_name: active.map(|p| p.current_branch.as_str()).unwrap_or(""),
            last_sync: active.map(|p| p.last_sync.as_str()).unwrap_or("never"),
            filter_mode: self.filter_mode,
            filter_text: &self.filter_text,
            author_filter_text: &self.author_filter_text,
            loading_message: loading_msg,
            commit_count,
            branch_count,
            palette: &self.palette,
        };
        frame.render_widget(status, area);
    }

    fn render_overlays(&self, frame: &mut Frame, size: Rect) {
        if self.show_help {
            if self.graphics_cap.is_kitty() {
                self.clear_kitty_images_in_area(frame, size);
            }
            frame.render_widget(
                HelpPanel {
                    palette: &self.palette,
                },
                size,
            );
        }
        if self.confirm_quit {
            use ratatui::text::{Line, Span};
            let dialog_area = crate::ui::centered_rect(30, 15, size);
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(self.palette.warn_fg))
                .title(" Quit ")
                .title_style(
                    Style::default()
                        .fg(self.palette.warn_fg)
                        .add_modifier(Modifier::BOLD),
                );
            let inner = block.inner(dialog_area);
            frame.render_widget(ratatui::widgets::Clear, dialog_area);
            frame.render_widget(block, dialog_area);
            if inner.height >= 2 && inner.width >= 10 {
                let msg = Line::from(Span::styled(
                    "Discard changes? (y/n)",
                    Style::default().fg(self.palette.content_fg),
                ));
                frame.buffer_mut().set_line(
                    inner.x + 1,
                    inner.y + 1,
                    &msg,
                    inner.width.saturating_sub(2),
                );
            }
        }
        if let Some(ref n) = self.notification {
            frame.render_widget(
                crate::ui::toast::Toast {
                    notification: n,
                    palette: &self.palette,
                },
                size,
            );
        }
    }

    fn clear_kitty_images_in_area(&self, frame: &mut Frame, area: Rect) {
        use ratatui::layout::Position;
        let del = crate::kitty_protocol::delete_all_kitty_images();
        if let Some(cell) = frame.buffer_mut().cell_mut(Position::new(area.x, area.y)) {
            cell.set_symbol(&del);
        }
    }

    fn ensure_scroll_bounds(&mut self, visible_height: usize) {
        if visible_height == 0 {
            return;
        }
        if self.graph_selected >= self.graph_scroll_y + visible_height {
            self.graph_scroll_y = self.graph_selected - visible_height + 1;
        }
        if self.graph_selected < self.graph_scroll_y {
            self.graph_scroll_y = self.graph_selected;
        }
    }

    fn get_highlighted_oids(&self, entries: &[DisplayEntry]) -> HashSet<crate::git::types::Oid> {
        let mut set = HashSet::new();
        if self.active_panel == Panel::Branches {
            if let Some(entry) = entries.get(self.branch_selected) {
                if let Some(tip) = entry.tip_oid() {
                    set.insert(tip);
                }
            }
        }
        set
    }
}

pub fn head_branch_name(data: &RepoData) -> String {
    data.branches
        .iter()
        .find(|b| b.is_head)
        .map(|b| b.name.clone())
        .unwrap_or_else(|| "detached".to_string())
}

pub fn expand_tilde(path: &std::path::Path) -> std::path::PathBuf {
    let s = path.to_string_lossy();
    if let Some(rest) = s.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return std::path::PathBuf::from(home).join(rest);
        }
    }
    path.to_path_buf()
}
