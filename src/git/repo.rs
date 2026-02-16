use crate::error::{ArachneError, Result};
use crate::git::types::*;
use chrono::TimeZone;
use git2::{BranchType, Repository, Sort};
use std::collections::HashSet;
use std::path::Path;

pub fn open_repo(path: &Path) -> Result<Repository> {
    Repository::discover(path).map_err(|_| ArachneError::NotARepo(path.display().to_string()))
}

pub fn read_repo(repo: &Repository) -> Result<RepoData> {
    let mut data = RepoData::default();

    data.branches = list_branches(repo)?;
    data.tags = list_tags(repo)?;
    data.head = resolve_head(repo);
    data.branch_tips = data.branches.iter().map(|b| b.tip).collect();
    data.commits = topo_walk(repo)?;

    Ok(data)
}

fn resolve_head(repo: &Repository) -> Option<Oid> {
    repo.head()
        .ok()
        .and_then(|r| r.target())
        .map(Oid::from_git2)
}

fn list_branches(repo: &Repository) -> Result<Vec<BranchInfo>> {
    let head_oid = resolve_head(repo);
    let mut out = Vec::new();

    for branch_result in repo.branches(Some(BranchType::Local))? {
        let (branch, _) = branch_result?;
        let name = branch.name()?.unwrap_or("???").to_string();
        let tip = branch
            .get()
            .target()
            .map(Oid::from_git2)
            .unwrap_or(Oid::zero());
        let is_head = head_oid.as_ref() == Some(&tip) && branch.is_head();
        out.push(BranchInfo {
            name,
            tip,
            is_head,
            source: CommitSource::Local,
        });
    }

    for branch_result in repo.branches(Some(BranchType::Remote))? {
        let (branch, _) = branch_result?;
        let name = branch.name()?.unwrap_or("???").to_string();
        let tip = branch
            .get()
            .target()
            .map(Oid::from_git2)
            .unwrap_or(Oid::zero());
        let remote_name = name.find('/')
            .map(|i| name[..i].to_string())
            .unwrap_or_else(|| "origin".to_string());
        out.push(BranchInfo {
            name,
            tip,
            is_head: false,
            source: CommitSource::Remote(remote_name),
        });
    }

    Ok(out)
}

fn list_tags(repo: &Repository) -> Result<Vec<TagInfo>> {
    let mut out = Vec::new();
    repo.tag_foreach(|oid, name_bytes| {
        let name = String::from_utf8_lossy(name_bytes)
            .strip_prefix("refs/tags/")
            .unwrap_or(&String::from_utf8_lossy(name_bytes))
            .to_string();
        let target = repo
            .find_tag(oid)
            .ok()
            .and_then(|tag| tag.target().ok())
            .map(|obj| Oid::from_git2(obj.id()))
            .unwrap_or(Oid::from_git2(oid));
        out.push(TagInfo { name, target });
        true
    })?;
    Ok(out)
}

fn topo_walk(repo: &Repository) -> Result<Vec<CommitInfo>> {
    let mut revwalk = repo.revwalk()?;
    revwalk.set_sorting(Sort::TOPOLOGICAL | Sort::TIME)?;

    let mut pushed = HashSet::new();
    for branch_result in repo.branches(None)? {
        let (branch, _) = branch_result?;
        if let Some(oid) = branch.get().target() {
            if pushed.insert(oid) {
                revwalk.push(oid)?;
            }
        }
    }

    repo.tag_foreach(|oid, _| {
        if let Ok(obj) = repo.revparse_single(&oid.to_string()) {
            if let Ok(commit) = obj.peel_to_commit() {
                let cid = commit.id();
                if pushed.insert(cid) {
                    let _ = revwalk.push(cid);
                }
            }
        }
        true
    })?;

    let mut commits = Vec::new();
    for oid_result in revwalk {
        let oid = oid_result?;
        let commit = repo.find_commit(oid)?;
        let time_secs = commit.time().seconds();
        let time = chrono::Utc
            .timestamp_opt(time_secs, 0)
            .single()
            .unwrap_or_default();

        commits.push(CommitInfo {
            oid: Oid::from_git2(oid),
            parents: commit.parent_ids().map(Oid::from_git2).collect(),
            message: commit.summary().unwrap_or("").to_string(),
            author: commit.author().name().unwrap_or("").to_string(),
            time,
            source: CommitSource::Local,
        });
    }

    Ok(commits)
}
