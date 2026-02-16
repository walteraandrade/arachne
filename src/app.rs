use crate::config::Config;
use crate::error::Result;
use crate::event::{AppEvent, GitHubData};
use crate::git::{repo, types::RepoData};
use crate::github::client::GitHubClient;
use crate::graph::{dag::Dag, layout, types::GraphRow};
use crate::ui::{
    branch_panel::{self, BranchPanel, DisplayEntry, SectionKey},
    detail_panel::DetailPanel,
    graph_view::GraphView,
    input::{self, Action, FilterMode},
    status_bar::StatusBar,
};
use git2::Repository;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq)]
pub enum Panel {
    Branches,
    Graph,
}

pub struct RepoPane {
    pub repo: Option<Repository>,
    pub repo_data: RepoData,
    pub dag: Dag,
    pub rows: Vec<GraphRow>,
    pub repo_name: String,
    pub current_branch: String,
    pub github_client: Option<GitHubClient>,
    pub scroll_x: usize,
    pub last_sync: String,
    pub rate_limit: Option<u32>,
}

pub struct App {
    pub config: Config,
    pub panes: Vec<RepoPane>,
    pub active_pane: usize,

    pub active_panel: Panel,
    pub graph_scroll_y: usize,
    pub graph_selected: usize,
    pub branch_scroll: usize,
    pub branch_selected: usize,

    pub show_detail: bool,
    pub show_forks: bool,
    pub filter_mode: FilterMode,
    pub filter_text: String,
    pub author_filter_text: String,
    pub collapsed_sections: HashSet<SectionKey>,

    pub should_quit: bool,
}

impl App {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            panes: Vec::new(),
            active_pane: 0,
            active_panel: Panel::Graph,
            graph_scroll_y: 0,
            graph_selected: 0,
            branch_scroll: 0,
            branch_selected: 0,
            show_detail: false,
            show_forks: true,
            filter_mode: FilterMode::Off,
            filter_text: String::new(),
            author_filter_text: String::new(),
            collapsed_sections: HashSet::new(),
            should_quit: false,
        }
    }

    pub fn load_repos(&mut self) -> Result<()> {
        let entries = self.config.resolved_repos();
        for entry in &entries {
            let path = expand_tilde(&entry.path);
            let r = repo::open_repo(&path)?;
            let repo_data = repo::read_repo(&r)?;

            let current_branch = repo_data
                .branches
                .iter()
                .find(|b| b.is_head)
                .map(|b| b.name.clone())
                .unwrap_or_else(|| "detached".to_string());

            let repo_name = entry
                .name
                .clone()
                .unwrap_or_else(|| detect_repo_name(&r));

            let dag = Dag::from_repo_data(&repo_data);
            let rows = layout::compute_layout(&dag, &repo_data, &self.config.trunk_branches);

            let github_client = init_github_client(&self.config, &repo_name);

            self.panes.push(RepoPane {
                repo: Some(r),
                repo_data,
                dag,
                rows,
                repo_name,
                current_branch,
                github_client,
                scroll_x: 0,
                last_sync: "just now".to_string(),
                rate_limit: None,
            });
        }
        self.collapsed_sections = branch_panel::auto_collapse_defaults(&self.panes, "");
        Ok(())
    }

    pub fn rebuild_graph(&mut self, pane_idx: usize) {
        if let Some(pane) = self.panes.get_mut(pane_idx) {
            if let Some(ref r) = pane.repo {
                if let Ok(mut data) = repo::read_repo(r) {
                    pane.current_branch = data
                        .branches
                        .iter()
                        .find(|b| b.is_head)
                        .map(|b| b.name.clone())
                        .unwrap_or_else(|| "detached".to_string());

                    if !self.author_filter_text.is_empty() {
                        filter_by_author(&mut data, &self.author_filter_text);
                    }

                    pane.repo_data = data;
                    pane.dag = Dag::from_repo_data(&pane.repo_data);
                    pane.rows = layout::compute_layout(&pane.dag, &pane.repo_data, &self.config.trunk_branches);
                    pane.last_sync = "just now".to_string();
                }
            }
        }
    }

    pub fn handle_github_result(&mut self, pane_idx: usize, result: std::result::Result<GitHubData, String>) {
        if let Some(pane) = self.panes.get_mut(pane_idx) {
            match result {
                Ok(data) => {
                    pane.rate_limit = data.rate_limit;
                    pane.repo_data.branches.extend(data.branches);
                    pane.dag.merge_remote(data.commits);
                    pane.rows = layout::compute_layout(&pane.dag, &pane.repo_data, &self.config.trunk_branches);
                    pane.last_sync = "just now".to_string();
                }
                Err(e) => {
                    pane.last_sync = format!("error: {e}");
                }
            }
        }
    }

    pub fn handle_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::Key(key) => {
                let action = input::map_key(key, self.filter_mode);
                self.handle_action(action);
            }
            AppEvent::FsChanged(idx) => self.rebuild_graph(idx),
            AppEvent::GitHubUpdate(_) => {}
            AppEvent::GitHubResult { pane_idx, result } => {
                self.handle_github_result(pane_idx, result);
            }
            AppEvent::Resize => {}
        }
    }

    fn handle_action(&mut self, action: Action) {
        match action {
            Action::Quit => self.should_quit = true,
            Action::ScrollDown => match self.active_panel {
                Panel::Graph => {
                    if let Some(pane) = self.panes.get(self.active_pane) {
                        if self.graph_selected + 1 < pane.rows.len() {
                            self.graph_selected += 1;
                            self.sync_pane_scroll();
                        }
                    }
                }
                Panel::Branches => {
                    let entries = branch_panel::build_entries(
                        &self.panes,
                        &self.filter_text,
                        self.show_forks,
                        &self.collapsed_sections,
                    );
                    if !entries.is_empty() {
                        self.branch_selected = (self.branch_selected + 1).min(entries.len() - 1);
                    }
                }
            },
            Action::ScrollUp => match self.active_panel {
                Panel::Graph => {
                    if self.graph_selected > 0 {
                        self.graph_selected = self.graph_selected.saturating_sub(1);
                        self.sync_pane_scroll();
                    }
                }
                Panel::Branches => {
                    self.branch_selected = self.branch_selected.saturating_sub(1);
                }
            },
            Action::ScrollLeft => {
                if let Some(pane) = self.panes.get_mut(self.active_pane) {
                    pane.scroll_x = pane.scroll_x.saturating_sub(4);
                }
            }
            Action::ScrollRight => {
                if let Some(pane) = self.panes.get_mut(self.active_pane) {
                    pane.scroll_x = pane.scroll_x.saturating_add(4);
                }
            }
            Action::PanelLeft => self.active_panel = Panel::Branches,
            Action::PanelRight => self.active_panel = Panel::Graph,
            Action::NextPane => {
                if !self.panes.is_empty() {
                    self.active_pane = (self.active_pane + 1) % self.panes.len();
                    self.clamp_selected();
                }
            }
            Action::PrevPane => {
                if !self.panes.is_empty() {
                    self.active_pane = if self.active_pane == 0 {
                        self.panes.len() - 1
                    } else {
                        self.active_pane - 1
                    };
                    self.clamp_selected();
                }
            }
            Action::Select => {
                if self.active_panel == Panel::Branches {
                    self.toggle_branch_section();
                } else {
                    self.show_detail = !self.show_detail;
                }
            }
            Action::ToggleForks => self.show_forks = !self.show_forks,
            Action::Filter => self.filter_mode = FilterMode::Branch,
            Action::AuthorFilter => self.filter_mode = FilterMode::Author,
            Action::FilterChar(c) => match self.filter_mode {
                FilterMode::Branch => self.filter_text.push(c),
                FilterMode::Author => self.author_filter_text.push(c),
                FilterMode::Off => {}
            },
            Action::FilterBackspace => match self.filter_mode {
                FilterMode::Branch => { self.filter_text.pop(); }
                FilterMode::Author => { self.author_filter_text.pop(); }
                FilterMode::Off => {}
            },
            Action::FilterConfirm => {
                let was_author = self.filter_mode == FilterMode::Author;
                self.filter_mode = FilterMode::Off;
                if was_author {
                    for idx in 0..self.panes.len() {
                        self.rebuild_graph(idx);
                    }
                    self.clamp_selected();
                }
            }
            Action::FilterCancel => {
                match self.filter_mode {
                    FilterMode::Branch => self.filter_text.clear(),
                    FilterMode::Author => {
                        self.author_filter_text.clear();
                        self.filter_mode = FilterMode::Off;
                        for idx in 0..self.panes.len() {
                            self.rebuild_graph(idx);
                        }
                        self.clamp_selected();
                        return;
                    }
                    FilterMode::Off => {}
                }
                self.filter_mode = FilterMode::Off;
            }
            Action::Refresh => {
                for idx in 0..self.panes.len() {
                    self.rebuild_graph(idx);
                }
            }
            Action::ClosePopup => self.show_detail = false,
            Action::None => {}
        }
    }

    fn clamp_selected(&mut self) {
        if let Some(pane) = self.panes.get(self.active_pane) {
            if !pane.rows.is_empty() && self.graph_selected >= pane.rows.len() {
                self.graph_selected = pane.rows.len() - 1;
            }
        }
    }

    fn toggle_branch_section(&mut self) {
        let entries = branch_panel::build_entries(
            &self.panes,
            &self.filter_text,
            self.show_forks,
            &self.collapsed_sections,
        );
        if let Some(entry) = entries.get(self.branch_selected) {
            if let Some(key) = entry.section_key() {
                if self.collapsed_sections.contains(key) {
                    self.collapsed_sections.remove(key);
                } else {
                    self.collapsed_sections.insert(key.clone());
                }
            }
        }
    }

    fn sync_pane_scroll(&self) {
        // Time-sync is handled during render via scroll_y_for_pane
    }

    fn scroll_y_for_pane(&self, pane_idx: usize, visible_height: usize) -> usize {
        if pane_idx == self.active_pane {
            return self.graph_scroll_y;
        }

        let active = match self.panes.get(self.active_pane) {
            Some(p) => p,
            None => return 0,
        };
        let target = match self.panes.get(pane_idx) {
            Some(p) => p,
            None => return 0,
        };

        let anchor_time = match active.rows.get(self.graph_selected) {
            Some(row) => row.time,
            None => return 0,
        };

        let closest_idx = find_closest_by_time(&target.rows, anchor_time);

        closest_idx.saturating_sub(visible_height / 2)
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let size = frame.area();

        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(size);

        let entries = branch_panel::build_entries(
            &self.panes,
            &self.filter_text,
            self.show_forks,
            &self.collapsed_sections,
        );
        let max_w = branch_panel::max_entry_width(&entries);
        let term_w = size.width as usize;
        let panel_w = (max_w + 2).clamp(20, term_w / 3) as u16;

        let body_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(panel_w), Constraint::Min(1)])
            .split(main_chunks[0]);

        let graph_area = body_chunks[1];
        let visible_height = graph_area.height as usize;

        self.ensure_scroll_bounds(visible_height);

        let highlighted: HashSet<_> = self.get_highlighted_oids(&entries);

        let branch_panel = BranchPanel {
            entries: &entries,
            selected: self.branch_selected,
            scroll: self.branch_scroll,
            focused: self.active_panel == Panel::Branches,
        };
        frame.render_widget(branch_panel, body_chunks[0]);

        let weights: Vec<u32> = self
            .panes
            .iter()
            .map(|p| ((p.rows.len() as f64).sqrt().ceil() as u32).max(1))
            .collect();
        let total: u32 = weights.iter().sum::<u32>().max(1);
        let constraints: Vec<Constraint> = weights
            .iter()
            .map(|&w| Constraint::Ratio(w, total))
            .collect();
        let pane_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(constraints)
            .split(graph_area);

        for (idx, pane) in self.panes.iter().enumerate() {
            let is_active = idx == self.active_pane;
            let pane_scroll_y = self.scroll_y_for_pane(idx, visible_height.saturating_sub(1));
            let selected = if is_active {
                self.graph_selected
            } else {
                find_closest_by_time(
                    &pane.rows,
                    self.panes
                        .get(self.active_pane)
                        .and_then(|p| p.rows.get(self.graph_selected))
                        .map(|r| r.time)
                        .unwrap_or_default(),
                )
            };

            let graph_view = GraphView {
                rows: &pane.rows,
                scroll_y: pane_scroll_y,
                scroll_x: pane.scroll_x,
                selected,
                highlighted_oids: &highlighted,
                repo_name: &pane.repo_name,
                is_active,
                is_first_pane: idx == 0,
                trunk_count: self.config.trunk_branches.len(),
            };
            frame.render_widget(graph_view, pane_chunks[idx]);
        }

        let pane_names: Vec<(&str, bool)> = self
            .panes
            .iter()
            .enumerate()
            .map(|(i, p)| (p.repo_name.as_str(), i == self.active_pane))
            .collect();

        let active = self.panes.get(self.active_pane);
        let status = StatusBar {
            pane_tabs: &pane_names,
            branch_name: active.map(|p| p.current_branch.as_str()).unwrap_or(""),
            last_sync: active.map(|p| p.last_sync.as_str()).unwrap_or("never"),
            rate_limit: active.and_then(|p| p.rate_limit),
            filter_mode: self.filter_mode,
            filter_text: &self.filter_text,
            author_filter_text: &self.author_filter_text,
        };
        frame.render_widget(status, main_chunks[1]);

        if self.show_detail {
            if let Some(pane) = self.panes.get(self.active_pane) {
                if let Some(row) = pane.rows.get(self.graph_selected) {
                    let detail = DetailPanel { row };
                    frame.render_widget(detail, size);
                }
            }
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

fn find_closest_by_time(
    rows: &[GraphRow],
    target: chrono::DateTime<chrono::Utc>,
) -> usize {
    if rows.is_empty() {
        return 0;
    }
    let mut best = 0;
    let mut best_diff = i64::MAX;
    for (i, row) in rows.iter().enumerate() {
        let diff = (row.time - target).num_seconds().abs();
        if diff < best_diff {
            best_diff = diff;
            best = i;
        }
    }
    best
}

fn init_github_client(config: &Config, repo_name: &str) -> Option<GitHubClient> {
    let token = config.github_token.as_ref()?;
    if token.is_empty() {
        return None;
    }
    let parts: Vec<&str> = repo_name.splitn(2, '/').collect();
    if parts.len() == 2 {
        GitHubClient::new(token, parts[0], parts[1]).ok()
    } else {
        None
    }
}

fn expand_tilde(path: &std::path::Path) -> std::path::PathBuf {
    let s = path.to_string_lossy();
    if let Some(rest) = s.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return std::path::PathBuf::from(home).join(rest);
        }
    }
    path.to_path_buf()
}

fn filter_by_author(data: &mut RepoData, author_query: &str) {
    use std::collections::HashMap;

    let query = author_query.to_lowercase();
    let commit_map: HashMap<_, _> = data.commits.iter().map(|c| (c.oid, c)).collect();

    let matching_branches: Vec<_> = data
        .branches
        .iter()
        .filter(|branch| {
            let mut current = Some(&branch.tip);
            while let Some(oid) = current {
                if let Some(commit) = commit_map.get(oid) {
                    if commit.author.to_lowercase().contains(&query) {
                        return true;
                    }
                    current = commit.parents.first();
                } else {
                    break;
                }
            }
            false
        })
        .cloned()
        .collect();

    let mut reachable: HashSet<crate::git::types::Oid> = HashSet::new();
    for branch in &matching_branches {
        let mut stack = vec![branch.tip];
        while let Some(oid) = stack.pop() {
            if !reachable.insert(oid) {
                continue;
            }
            if let Some(commit) = commit_map.get(&oid) {
                stack.extend(commit.parents.iter().copied());
            }
        }
    }

    data.branches = matching_branches;
    data.commits.retain(|c| reachable.contains(&c.oid));
    data.branch_tips = data.branches.iter().map(|b| b.tip).collect();
}

fn detect_repo_name(repo: &Repository) -> String {
    repo.find_remote("origin")
        .ok()
        .and_then(|remote| remote.url().map(String::from))
        .and_then(|url| {
            let url = url.trim_end_matches(".git");
            if url.contains("github.com") {
                let parts: Vec<&str> = url.rsplitn(3, '/').collect();
                if parts.len() >= 2 {
                    return Some(format!("{}/{}", parts[1], parts[0]));
                }
            }
            url.rsplit('/').next().map(String::from)
        })
        .unwrap_or_else(|| {
            repo.workdir()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string())
        })
}
