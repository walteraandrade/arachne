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
    header_bar::{HeaderBar, PaneInfo},
    help_panel::HelpPanel,
    input::{self, Action, FilterMode},
    status_bar::StatusBar,
    theme,
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
    pub time_sorted_indices: Vec<usize>,
    pub cached_repo_data: Option<RepoData>,
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
    pub show_help: bool,
    pub show_forks: bool,
    pub filter_mode: FilterMode,
    pub filter_text: String,
    pub author_filter_text: String,
    pub collapsed_sections: HashSet<SectionKey>,
    pub error_message: Option<(String, std::time::Instant)>,

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
            show_help: false,
            show_forks: true,
            filter_mode: FilterMode::Off,
            filter_text: String::new(),
            author_filter_text: String::new(),
            collapsed_sections: HashSet::new(),
            error_message: None,
            should_quit: false,
        }
    }

    pub fn load_repos(&mut self) -> Result<()> {
        let entries = self.config.resolved_repos();
        for entry in &entries {
            let path = expand_tilde(&entry.path);
            let r = repo::open_repo(&path)?;
            let repo_data = repo::read_repo(&r, self.config.max_commits)?;

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

            let time_sorted_indices = build_time_sorted_indices(&rows);
            let cached_repo_data = Some(repo_data.clone());
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
                time_sorted_indices,
                cached_repo_data,
            });
        }
        self.collapsed_sections = branch_panel::auto_collapse_defaults(&self.panes, "");
        Ok(())
    }

    pub fn rebuild_graph(&mut self, pane_idx: usize) {
        self.rebuild_graph_inner(pane_idx, false);
    }

    pub fn rebuild_graph_author_only(&mut self, pane_idx: usize) {
        self.rebuild_graph_inner(pane_idx, true);
    }

    fn rebuild_graph_inner(&mut self, pane_idx: usize, author_only: bool) {
        if let Some(pane) = self.panes.get_mut(pane_idx) {
            let mut data = if author_only {
                if let Some(ref cached) = pane.cached_repo_data {
                    cached.clone()
                } else {
                    return;
                }
            } else if let Some(ref r) = pane.repo {
                match repo::read_repo(r, self.config.max_commits) {
                    Ok(d) => {
                        pane.cached_repo_data = Some(d.clone());
                        d
                    }
                    Err(_) => return,
                }
            } else {
                return;
            };

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
            pane.time_sorted_indices = build_time_sorted_indices(&pane.rows);
            pane.last_sync = "just now".to_string();
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
                    pane.time_sorted_indices = build_time_sorted_indices(&pane.rows);
                    pane.cached_repo_data = None; // invalidate cache on remote data
                    pane.last_sync = "just now".to_string();
                    self.error_message = None;
                }
                Err(e) => {
                    pane.last_sync = format!("error: {e}");
                    self.error_message = Some((e, std::time::Instant::now()));
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
                        let mut next = self.branch_selected + 1;
                        while next < entries.len() && entries[next].is_spacer() {
                            next += 1;
                        }
                        if next < entries.len() {
                            self.branch_selected = next;
                        }
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
                    let entries = branch_panel::build_entries(
                        &self.panes,
                        &self.filter_text,
                        self.show_forks,
                        &self.collapsed_sections,
                    );
                    if self.branch_selected > 0 {
                        let mut prev = self.branch_selected - 1;
                        while prev > 0 && entries[prev].is_spacer() {
                            prev -= 1;
                        }
                        if !entries[prev].is_spacer() {
                            self.branch_selected = prev;
                        }
                    }
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
                        self.rebuild_graph_author_only(idx);
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
                            self.rebuild_graph_author_only(idx);
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
            Action::Help => self.show_help = !self.show_help,
            Action::ClosePopup => {
                self.show_detail = false;
                self.show_help = false;
            }
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
            } else if let Some(tip) = entry.tip_oid() {
                if let Some(pane) = self.panes.get(self.active_pane) {
                    if let Some(idx) = pane.rows.iter().position(|r| r.oid == tip) {
                        self.graph_selected = idx;
                    }
                }
            }
        }
    }

    fn sync_pane_scroll(&self) {
        // Time-sync is handled during render via scroll_y_for_pane
    }

    fn pane_sync_info(&self, pane_idx: usize, visible_height: usize) -> (usize, usize) {
        if pane_idx == self.active_pane {
            return (self.graph_scroll_y, self.graph_selected);
        }

        let anchor_time = self
            .panes
            .get(self.active_pane)
            .and_then(|p| p.rows.get(self.graph_selected))
            .map(|r| r.time);

        let anchor_time = match anchor_time {
            Some(t) => t,
            None => return (0, 0),
        };

        let target = match self.panes.get(pane_idx) {
            Some(p) => p,
            None => return (0, 0),
        };

        let closest_idx = find_closest_by_time_binary(&target.rows, &target.time_sorted_indices, anchor_time);
        let scroll_y = closest_idx.saturating_sub(visible_height / 2);
        (scroll_y, closest_idx)
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let size = frame.area();

        // Vertical: header(1) + separator(1) + body(min) + status(1)
        let vert = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // header bar
                Constraint::Length(1), // horizontal separator
                Constraint::Min(1),    // body
                Constraint::Length(1), // status bar
            ])
            .split(size);

        let header_area = vert[0];
        let hsep_area = vert[1];
        let body_area = vert[2];
        let status_area = vert[3];

        // Header bar
        {
            let pane_infos: Vec<PaneInfo<'_>> = self
                .panes
                .iter()
                .enumerate()
                .map(|(i, p)| PaneInfo {
                    name: &p.repo_name,
                    branch: &p.current_branch,
                    is_active: i == self.active_pane,
                    commit_count: p.rows.len(),
                })
                .collect();
            let last_sync = self.panes.get(self.active_pane)
                .map(|p| p.last_sync.as_str())
                .unwrap_or("never");
            let header = HeaderBar {
                panes: &pane_infos,
                last_sync,
                author_filter: &self.author_filter_text,
            };
            frame.render_widget(header, header_area);
        }

        // Build entries for branch panel width calculation
        let entries = branch_panel::build_entries(
            &self.panes,
            &self.filter_text,
            self.show_forks,
            &self.collapsed_sections,
        );
        let max_w = branch_panel::max_entry_width(&entries);
        let term_w = size.width as usize;
        let panel_w = (max_w + 2).clamp(20, term_w / 3) as u16;

        // Horizontal body: branch_panel | separator(1) | graph_area
        let body_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(panel_w),
                Constraint::Length(1), // vertical separator
                Constraint::Min(1),
            ])
            .split(body_area);

        let branch_area = body_chunks[0];
        let vsep_area = body_chunks[1];
        let graph_area = body_chunks[2];

        // Horizontal separator across full width
        let sep_style = ratatui::style::Style::default().fg(theme::SEPARATOR);
        for x in hsep_area.x..hsep_area.right() {
            frame.buffer_mut()[(x, hsep_area.y)].set_char('\u{2500}');
            frame.buffer_mut()[(x, hsep_area.y)].set_style(sep_style);
        }
        // Cross junction at vertical separator position
        if vsep_area.x > hsep_area.x && vsep_area.x < hsep_area.right() {
            frame.buffer_mut()[(vsep_area.x, hsep_area.y)].set_char('\u{253c}');
        }

        // Vertical separator
        for y in vsep_area.y..vsep_area.bottom() {
            frame.buffer_mut()[(vsep_area.x, y)].set_char('\u{2503}');
            frame.buffer_mut()[(vsep_area.x, y)].set_style(sep_style);
        }

        let visible_height = graph_area.height as usize;
        self.ensure_scroll_bounds(visible_height);

        let highlighted: HashSet<_> = self.get_highlighted_oids(&entries);

        // Branch panel scroll
        let branch_visible = branch_area.height.saturating_sub(2) as usize;
        if branch_visible > 0 {
            if self.branch_selected >= self.branch_scroll + branch_visible {
                self.branch_scroll = self.branch_selected - branch_visible + 1;
            }
            if self.branch_selected < self.branch_scroll {
                self.branch_scroll = self.branch_selected;
            }
        }

        let branch_panel = BranchPanel {
            entries: &entries,
            selected: self.branch_selected,
            scroll: self.branch_scroll,
            focused: self.active_panel == Panel::Branches,
        };
        frame.render_widget(branch_panel, branch_area);

        // Graph panes
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

        let graph_focused = self.active_panel == Panel::Graph;
        for (idx, pane) in self.panes.iter().enumerate() {
            let is_active_pane = idx == self.active_pane;
            let (pane_scroll_y, selected) =
                self.pane_sync_info(idx, visible_height);

            let graph_view = GraphView {
                rows: &pane.rows,
                scroll_y: pane_scroll_y,
                scroll_x: pane.scroll_x,
                selected,
                highlighted_oids: &highlighted,
                is_active: is_active_pane && graph_focused,
                is_first_pane: idx == 0,
                trunk_count: self.config.trunk_branches.len(),
            };
            frame.render_widget(graph_view, pane_chunks[idx]);
        }

        // Auto-dismiss error after 30s
        if let Some((_, instant)) = &self.error_message {
            if instant.elapsed() > std::time::Duration::from_secs(30) {
                self.error_message = None;
            }
        }

        // Status bar
        let active = self.panes.get(self.active_pane);
        let error_msg = self.error_message.as_ref().map(|(msg, _)| msg.as_str());
        let commit_count = active.map(|p| p.rows.len()).unwrap_or(0);
        let branch_count = active.map(|p| p.repo_data.branches.len()).unwrap_or(0);
        let status = StatusBar {
            branch_name: active.map(|p| p.current_branch.as_str()).unwrap_or(""),
            last_sync: active.map(|p| p.last_sync.as_str()).unwrap_or("never"),
            filter_mode: self.filter_mode,
            filter_text: &self.filter_text,
            author_filter_text: &self.author_filter_text,
            error_message: error_msg,
            commit_count,
            branch_count,
        };
        frame.render_widget(status, status_area);

        if self.show_detail {
            if let Some(pane) = self.panes.get(self.active_pane) {
                if let Some(row) = pane.rows.get(self.graph_selected) {
                    let detail = DetailPanel { row };
                    frame.render_widget(detail, size);
                }
            }
        }

        if self.show_help {
            frame.render_widget(HelpPanel, size);
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

fn build_time_sorted_indices(rows: &[GraphRow]) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..rows.len()).collect();
    indices.sort_by(|&a, &b| rows[b].time.cmp(&rows[a].time));
    indices
}

fn find_closest_by_time_binary(
    rows: &[GraphRow],
    sorted_indices: &[usize],
    target: chrono::DateTime<chrono::Utc>,
) -> usize {
    if rows.is_empty() {
        return 0;
    }
    // sorted_indices is sorted by time descending
    let pos = sorted_indices.partition_point(|&i| rows[i].time > target);
    if pos == 0 {
        return sorted_indices[0];
    }
    if pos >= sorted_indices.len() {
        return *sorted_indices.last().unwrap();
    }
    let before = sorted_indices[pos - 1];
    let after = sorted_indices[pos];
    let diff_before = (rows[before].time - target).num_seconds().abs();
    let diff_after = (rows[after].time - target).num_seconds().abs();
    if diff_before <= diff_after { before } else { after }
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
    use std::collections::{HashMap, VecDeque};

    let query = author_query.to_lowercase();
    let commit_map: HashMap<_, _> = data.commits.iter().map(|c| (c.oid, c)).collect();

    // Step 1: matching commits by author substring
    let matching: HashSet<crate::git::types::Oid> = data
        .commits
        .iter()
        .filter(|c| c.author.to_lowercase().contains(&query))
        .map(|c| c.oid)
        .collect();

    // Step 2: branches reachable to at least one match via full BFS (all parents)
    let matching_branches: Vec<_> = data
        .branches
        .iter()
        .filter(|branch| {
            let mut visited = HashSet::new();
            let mut queue = VecDeque::new();
            queue.push_back(branch.tip);
            while let Some(oid) = queue.pop_front() {
                if !visited.insert(oid) {
                    continue;
                }
                if matching.contains(&oid) {
                    return true;
                }
                if let Some(commit) = commit_map.get(&oid) {
                    queue.extend(commit.parents.iter().copied());
                }
            }
            false
        })
        .cloned()
        .collect();

    // Step 3: skip cache — for each non-matching commit, its nearest matching ancestors.
    // Process in reverse topo order (parents before children) so deps are ready.
    let mut skip_cache: HashMap<crate::git::types::Oid, Vec<crate::git::types::Oid>> =
        HashMap::new();
    for commit in data.commits.iter().rev() {
        if matching.contains(&commit.oid) {
            skip_cache.insert(commit.oid, vec![commit.oid]);
        } else {
            let mut ancestors = Vec::new();
            let mut seen = HashSet::new();
            for &parent in &commit.parents {
                if let Some(parent_ancestors) = skip_cache.get(&parent) {
                    for &a in parent_ancestors {
                        if seen.insert(a) {
                            ancestors.push(a);
                        }
                    }
                }
            }
            skip_cache.insert(commit.oid, ancestors);
        }
    }

    // Step 4: rewrite parents on matching commits — skip over filtered-out commits
    for commit in &mut data.commits {
        if !matching.contains(&commit.oid) {
            continue;
        }
        let mut new_parents = Vec::new();
        let mut seen = HashSet::new();
        for &parent in &commit.parents {
            if matching.contains(&parent) {
                if seen.insert(parent) {
                    new_parents.push(parent);
                }
            } else if let Some(ancestors) = skip_cache.get(&parent) {
                for &a in ancestors {
                    if seen.insert(a) {
                        new_parents.push(a);
                    }
                }
            }
        }
        commit.parents = new_parents;
    }

    // Step 5: retain only matching commits
    data.commits.retain(|c| matching.contains(&c.oid));

    // Step 6: retarget branch tips to nearest matching commit
    data.branches = matching_branches
        .into_iter()
        .filter_map(|mut branch| {
            if matching.contains(&branch.tip) {
                Some(branch)
            } else {
                let ancestors = skip_cache.get(&branch.tip)?;
                branch.tip = *ancestors.first()?;
                Some(branch)
            }
        })
        .collect();

    // Step 7: rebuild branch_tips
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
