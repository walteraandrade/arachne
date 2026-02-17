use crate::git::types::{Oid, RepoData};
use crate::github::client::GitHubClient;
use std::collections::HashSet;

pub async fn load_remote_repo_data(
    client: &GitHubClient,
    max_commits: usize,
) -> std::result::Result<RepoData, String> {
    let forks = client.fetch_forks().await.map_err(|e| e.to_string())?;

    let mut all_branches = Vec::new();
    let mut all_commits = Vec::new();
    let mut seen_oids: HashSet<Oid> = HashSet::new();

    for fork in &forks {
        let branches = match client.fetch_fork_branches(fork).await {
            Ok(b) => b,
            Err(_) => continue,
        };

        for branch in &branches {
            if all_commits.len() >= max_commits {
                break;
            }
            let remaining = max_commits.saturating_sub(all_commits.len());
            match client
                .fetch_commits(&fork.owner, &fork.repo, &branch.tip.to_string(), remaining.min(100))
                .await
            {
                Ok(commits) => {
                    for c in commits {
                        if seen_oids.insert(c.oid) {
                            all_commits.push(c);
                        }
                    }
                }
                Err(_) => continue,
            }
        }

        all_branches.extend(branches);
        if all_commits.len() >= max_commits {
            break;
        }
    }

    let branch_tips: HashSet<Oid> = all_branches.iter().map(|b| b.tip).collect();
    let head = all_branches.first().map(|b| b.tip);

    Ok(RepoData {
        commits: all_commits,
        branches: all_branches,
        tags: Vec::new(),
        head,
        branch_tips,
    })
}
