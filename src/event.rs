use crate::git::types::{BranchInfo, CommitInfo, RepoData};
use crossterm::event::KeyEvent;

#[derive(Debug)]
pub struct GitHubData {
    pub rate_limit: Option<u32>,
    pub branches: Vec<BranchInfo>,
    pub commits: Vec<CommitInfo>,
}

#[derive(Debug)]
pub enum AppEvent {
    Key(KeyEvent),
    Resize,
    FsChanged(usize),
    GitHubUpdate(usize),
    GitHubResult {
        project_idx: usize,
        result: std::result::Result<GitHubData, String>,
    },
    RemoteDataResult {
        project_idx: usize,
        result: std::result::Result<RepoData, String>,
    },
}
