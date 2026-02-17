use crate::data_source::{LocalSource, RemoteSource, ViewMode};
use crate::git::types::RepoData;
use crate::github::client::GitHubClient;
use crate::graph::{dag::Dag, layout, types::GraphRow};
use std::collections::HashMap;

pub struct Project {
    pub name: String,
    pub local_source: Option<LocalSource>,
    pub remote_source: Option<RemoteSource>,
    pub active_mode: ViewMode,
    pub repo_data: RepoData,
    pub dag: Dag,
    pub rows: Vec<GraphRow>,
    pub branch_index_to_name: HashMap<usize, String>,
    pub trunk_count: usize,
    pub current_branch: String,
    pub scroll_x: usize,
    pub last_sync: String,
    pub rate_limit: Option<u32>,
    pub time_sorted_indices: Vec<usize>,
    pub cached_repo_data: Option<RepoData>,
}

impl Project {
    pub fn github_client(&self) -> Option<&GitHubClient> {
        self.remote_source.as_ref().map(|s| &s.client)
    }

    pub fn rebuild_layout(&mut self, trunk_branches: &[String]) {
        self.dag = Dag::from_repo_data(&self.repo_data);
        let result = layout::compute_layout(&self.dag, &self.repo_data, trunk_branches);
        self.rows = result.rows;
        self.branch_index_to_name = result.branch_index_to_name;
        self.trunk_count = result.trunk_count;
        self.time_sorted_indices = build_time_sorted_indices(&self.rows);
    }
}

pub fn build_time_sorted_indices(rows: &[GraphRow]) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..rows.len()).collect();
    indices.sort_by(|&a, &b| {
        let ta = rows.get(a).map(|r| &r.time);
        let tb = rows.get(b).map(|r| &r.time);
        tb.cmp(&ta)
    });
    indices
}
