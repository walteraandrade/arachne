use crate::config::Config;
use crate::error::Result;
use crate::event::AppEvent;
use crate::git::{repo, types::RepoData};
use crate::github::client::GitHubClient;
use crate::github::network;
use crate::graph::{dag::Dag, layout, types::GraphRow};
use crate::ui::{
    branch_panel::BranchPanel,
    detail_panel::DetailPanel,
    graph_view::GraphView,
    input::{self, Action},
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

pub struct App {
    pub config: Config,
    pub repo: Option<Repository>,
    pub repo_data: RepoData,
    pub dag: Dag,
    pub rows: Vec<GraphRow>,
    pub repo_name: String,
    pub current_branch: String,

    pub active_panel: Panel,
    pub graph_scroll_y: usize,
    pub graph_scroll_x: usize,
    pub graph_selected: usize,
    pub branch_scroll: usize,
    pub branch_selected: usize,

    pub show_detail: bool,
    pub show_forks: bool,
    pub filter_mode: bool,
    pub filter_text: String,

    pub last_sync: String,
    pub rate_limit: Option<u32>,
    pub github_client: Option<GitHubClient>,

    pub should_quit: bool,
}

impl App {
    pub fn new(config: Config) -> Self {
        let github_client = config.github_token.as_ref().and_then(|token| {
            if token.is_empty() {
                return None;
            }
            None // will be set after repo detection
        });

        Self {
            config,
            repo: None,
            repo_data: RepoData::default(),
            dag: Dag::default(),
            rows: Vec::new(),
            repo_name: String::new(),
            current_branch: String::new(),
            active_panel: Panel::Graph,
            graph_scroll_y: 0,
            graph_scroll_x: 0,
            graph_selected: 0,
            branch_scroll: 0,
            branch_selected: 0,
            show_detail: false,
            show_forks: true,
            filter_mode: false,
            filter_text: String::new(),
            last_sync: "never".to_string(),
            rate_limit: None,
            github_client,
            should_quit: false,
        }
    }

    pub fn load_repo(&mut self) -> Result<()> {
        let r = repo::open_repo(&self.config.repo_path)?;
        self.repo_data = repo::read_repo(&r)?;

        self.current_branch = self
            .repo_data
            .branches
            .iter()
            .find(|b| b.is_head)
            .map(|b| b.name.clone())
            .unwrap_or_else(|| "detached".to_string());

        self.repo_name = detect_repo_name(&r);
        self.dag = Dag::from_repo_data(&self.repo_data);
        self.rows = layout::compute_layout(&self.dag, &self.repo_data);

        self.init_github_client();
        self.repo = Some(r);
        self.last_sync = "just now".to_string();
        Ok(())
    }

    fn init_github_client(&mut self) {
        if let Some(ref token) = self.config.github_token {
            if !token.is_empty() {
                let parts: Vec<&str> = self.repo_name.splitn(2, '/').collect();
                if parts.len() == 2 {
                    self.github_client = GitHubClient::new(token, parts[0], parts[1]).ok();
                }
            }
        }
    }

    pub fn rebuild_graph(&mut self) {
        if let Some(ref r) = self.repo {
            if let Ok(data) = repo::read_repo(r) {
                self.repo_data = data;
                self.dag = Dag::from_repo_data(&self.repo_data);
                self.rows = layout::compute_layout(&self.dag, &self.repo_data);
                self.last_sync = "just now".to_string();

                self.current_branch = self
                    .repo_data
                    .branches
                    .iter()
                    .find(|b| b.is_head)
                    .map(|b| b.name.clone())
                    .unwrap_or_else(|| "detached".to_string());
            }
        }
    }

    pub async fn fetch_github(&mut self) {
        if let Some(ref client) = self.github_client {
            let result = network::fetch_network(client, &mut self.dag, &mut self.repo_data).await;
            match result {
                Ok(rate) => {
                    self.rate_limit = rate;
                    self.rows = layout::compute_layout(&self.dag, &self.repo_data);
                }
                Err(e) => {
                    self.last_sync = format!("error: {e}");
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
            AppEvent::FsChanged => self.rebuild_graph(),
            AppEvent::GitHubUpdate => {}
            AppEvent::Resize(_, _) => {}
            AppEvent::Tick => {}
            AppEvent::Error(msg) => self.last_sync = format!("error: {msg}"),
        }
    }

    fn handle_action(&mut self, action: Action) {
        match action {
            Action::Quit => self.should_quit = true,
            Action::ScrollDown => match self.active_panel {
                Panel::Graph => {
                    if self.graph_selected + 1 < self.rows.len() {
                        self.graph_selected += 1;
                    }
                }
                Panel::Branches => self.branch_selected += 1,
            },
            Action::ScrollUp => match self.active_panel {
                Panel::Graph => {
                    self.graph_selected = self.graph_selected.saturating_sub(1);
                }
                Panel::Branches => {
                    self.branch_selected = self.branch_selected.saturating_sub(1);
                }
            },
            Action::ScrollLeft => {
                self.graph_scroll_x = self.graph_scroll_x.saturating_sub(4);
            }
            Action::ScrollRight => {
                self.graph_scroll_x += 4;
            }
            Action::PanelLeft => self.active_panel = Panel::Branches,
            Action::PanelRight => self.active_panel = Panel::Graph,
            Action::Select => {
                self.show_detail = !self.show_detail;
            }
            Action::ToggleForks => self.show_forks = !self.show_forks,
            Action::Filter => self.filter_mode = true,
            Action::FilterChar(c) => self.filter_text.push(c),
            Action::FilterBackspace => {
                self.filter_text.pop();
            }
            Action::FilterConfirm | Action::FilterCancel => {
                self.filter_mode = false;
                if matches!(action, Action::FilterCancel) {
                    self.filter_text.clear();
                }
            }
            Action::Refresh => self.rebuild_graph(),
            Action::ClosePopup => self.show_detail = false,
            Action::None => {}
        }
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let size = frame.area();

        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(size);

        let body_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(25), Constraint::Min(1)])
            .split(main_chunks[0]);

        self.ensure_scroll_bounds(body_chunks[1].height as usize);

        let highlighted: HashSet<_> = self.get_highlighted_oids();

        let branch_panel = BranchPanel {
            branches: &self.repo_data.branches,
            tags: &self.repo_data.tags,
            selected: self.branch_selected,
            scroll: self.branch_scroll,
            filter: &self.filter_text,
            focused: self.active_panel == Panel::Branches,
            show_forks: self.show_forks,
        };
        frame.render_widget(branch_panel, body_chunks[0]);

        let graph_view = GraphView {
            rows: &self.rows,
            scroll_y: self.graph_scroll_y,
            scroll_x: self.graph_scroll_x,
            selected: self.graph_selected,
            highlighted_oids: &highlighted,
        };
        frame.render_widget(graph_view, body_chunks[1]);

        let status = StatusBar {
            repo_name: &self.repo_name,
            branch_name: &self.current_branch,
            last_sync: &self.last_sync,
            rate_limit: self.rate_limit,
            filter_mode: self.filter_mode,
            filter_text: &self.filter_text,
        };
        frame.render_widget(status, main_chunks[1]);

        if self.show_detail {
            if let Some(row) = self.rows.get(self.graph_selected) {
                let detail = DetailPanel { row };
                frame.render_widget(detail, size);
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

    fn get_highlighted_oids(&self) -> HashSet<crate::git::types::Oid> {
        let mut set = HashSet::new();
        if self.active_panel == Panel::Branches {
            if let Some(branch) = self.repo_data.branches.get(self.branch_selected) {
                set.insert(branch.tip.clone());
            }
        }
        set
    }
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
