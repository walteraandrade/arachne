use crate::error::Result;
use crate::git::types::RepoData;
use crate::github::client::GitHubClient;
use crate::graph::dag::Dag;

pub async fn fetch_network(
    client: &GitHubClient,
    dag: &mut Dag,
    repo_data: &mut RepoData,
) -> Result<Option<u32>> {
    let forks = client.fetch_forks().await?;
    let mut all_remote_commits = Vec::new();

    for fork in &forks {
        let branches = client.fetch_fork_branches(fork).await?;

        for branch in &branches {
            let commits = client
                .fetch_commits(&fork.owner, &fork.repo, &branch.tip.to_string(), 100)
                .await
                .unwrap_or_default();
            all_remote_commits.extend(commits);
        }

        repo_data.branches.extend(branches);
    }

    dag.merge_remote(all_remote_commits);

    let rate = client.rate_limit().await;
    Ok(rate)
}
