use crate::config::Config;
use crate::github::client::GitHubClient;
use git2::Repository;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Local,
    Remote,
}

pub struct LocalSource {
    pub repo: Repository,
}

pub struct RemoteSource {
    pub client: GitHubClient,
}

pub fn init_github_client(config: &Config, repo_name: &str) -> Option<GitHubClient> {
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
