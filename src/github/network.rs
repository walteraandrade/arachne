use crate::git::types::{BranchInfo, CommitInfo};
use crate::github::client::GitHubClient;

pub async fn fetch_network_detached(
    client: &GitHubClient,
) -> std::result::Result<(Vec<BranchInfo>, Vec<CommitInfo>, Option<u32>), String> {
    let forks = client.fetch_forks().await.map_err(|e| e.to_string())?;
    let mut all_branches = Vec::new();
    let mut all_commits = Vec::new();

    for fork in &forks {
        let branches = match client.fetch_fork_branches(fork).await {
            Ok(b) => b,
            Err(e) => {
                eprintln!("warning: fetching branches for {}/{}: {e}", fork.owner, fork.repo);
                continue;
            }
        };

        for branch in &branches {
            match client
                .fetch_commits(&fork.owner, &fork.repo, &branch.tip.to_string(), 100)
                .await
            {
                Ok(commits) => all_commits.extend(commits),
                Err(e) => {
                    eprintln!("warning: fetching commits for {}/{} {}: {e}", fork.owner, fork.repo, branch.name);
                }
            }
        }

        all_branches.extend(branches);
    }

    let rate = client.rate_limit().await;
    Ok((all_branches, all_commits, rate))
}
