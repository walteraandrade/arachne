#[derive(Clone, Debug)]
pub struct ForkInfo {
    pub owner: String,
    pub repo: String,
    pub default_branch: String,
}

#[derive(Clone, Debug)]
pub struct RemoteBranchInfo {
    pub name: String,
    pub sha: String,
    pub fork_owner: String,
}

#[derive(Clone, Debug, Default)]
pub struct GitHubData {
    pub forks: Vec<ForkInfo>,
    pub rate_limit_remaining: Option<u32>,
}
