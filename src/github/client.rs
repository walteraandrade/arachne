use crate::error::{ArachneError, Result};
use crate::git::types::{BranchInfo, CommitInfo, CommitSource, Oid};
use crate::github::types::*;
use chrono::{DateTime, Utc};
use octocrab::Octocrab;

pub struct GitHubClient {
    octo: Octocrab,
    owner: String,
    repo: String,
}

impl GitHubClient {
    pub fn new(token: &str, owner: &str, repo: &str) -> Result<Self> {
        let octo = Octocrab::builder()
            .personal_token(token.to_string())
            .build()
            .map_err(|e| ArachneError::GitHub(e.to_string()))?;

        Ok(Self {
            octo,
            owner: owner.to_string(),
            repo: repo.to_string(),
        })
    }

    pub async fn fetch_forks(&self) -> Result<Vec<ForkInfo>> {
        let mut forks = Vec::new();
        let mut page = 1u32;

        loop {
            let result = self
                .octo
                .repos(&self.owner, &self.repo)
                .list_forks()
                .per_page(100)
                .page(page)
                .send()
                .await
                .map_err(|e| ArachneError::GitHub(e.to_string()))?;

            if result.items.is_empty() {
                break;
            }

            for fork in &result.items {
                let owner = fork
                    .owner
                    .as_ref()
                    .map(|o| o.login.clone())
                    .unwrap_or_default();
                let name = fork.name.clone();
                let default = fork
                    .default_branch
                    .clone()
                    .unwrap_or_else(|| "main".to_string());

                forks.push(ForkInfo {
                    owner,
                    repo: name,
                    default_branch: default,
                });
            }

            if result.next.is_none() {
                break;
            }
            page += 1;
        }

        Ok(forks)
    }

    pub async fn fetch_fork_branches(&self, fork: &ForkInfo) -> Result<Vec<BranchInfo>> {
        let mut branches = Vec::new();

        let result = self
            .octo
            .repos(&fork.owner, &fork.repo)
            .list_branches()
            .per_page(100)
            .send()
            .await
            .map_err(|e| ArachneError::GitHub(e.to_string()))?;

        for branch in &result.items {
            let sha_bytes = sha_str_to_bytes(&branch.commit.sha);
            branches.push(BranchInfo {
                name: branch.name.clone(),
                tip: Oid(sha_bytes),
                is_head: false,
                source: CommitSource::Fork(fork.owner.clone()),
            });
        }

        Ok(branches)
    }

    pub async fn fetch_commits(
        &self,
        owner: &str,
        repo: &str,
        sha: &str,
        max: usize,
    ) -> Result<Vec<CommitInfo>> {
        let mut commits = Vec::new();
        let mut page = 1u32;

        while commits.len() < max {
            let result = self
                .octo
                .repos(owner, repo)
                .list_commits()
                .sha(sha)
                .per_page(100)
                .page(page)
                .send()
                .await
                .map_err(|e| ArachneError::GitHub(e.to_string()))?;

            if result.items.is_empty() {
                break;
            }

            for c in &result.items {
                let oid = Oid(sha_str_to_bytes(&c.sha));
                let parents: Vec<Oid> = c
                    .parents
                    .iter()
                    .filter_map(|p| p.sha.as_deref())
                    .map(|sha| Oid(sha_str_to_bytes(sha)))
                    .collect();

                let message = c.commit.message.lines().next().unwrap_or("").to_string();

                let author = c
                    .commit
                    .author
                    .as_ref()
                    .map(|a| a.name.clone())
                    .unwrap_or_default();

                let time: DateTime<Utc> = c
                    .commit
                    .author
                    .as_ref()
                    .and_then(|a| a.date.as_ref())
                    .cloned()
                    .unwrap_or_default();

                commits.push(CommitInfo {
                    oid,
                    parents,
                    message,
                    author,
                    time,
                    source: CommitSource::Fork(owner.to_string()),
                });
            }

            if result.next.is_none() || commits.len() >= max {
                break;
            }
            page += 1;
        }

        Ok(commits)
    }

    pub async fn rate_limit(&self) -> Option<u32> {
        self.octo
            .ratelimit()
            .get()
            .await
            .ok()
            .map(|r| r.rate.remaining as u32)
    }
}

fn sha_str_to_bytes(sha: &str) -> [u8; 20] {
    let mut bytes = [0u8; 20];
    for (i, chunk) in sha.as_bytes().chunks(2).take(20).enumerate() {
        if let Ok(s) = std::str::from_utf8(chunk) {
            if let Ok(b) = u8::from_str_radix(s, 16) {
                bytes[i] = b;
            }
        }
    }
    bytes
}
