#![cfg(test)]

use crate::git::types::*;

pub fn make_oid(val: u8) -> Oid {
    let mut bytes = [0u8; 20];
    bytes[0] = val;
    Oid::from_bytes(bytes)
}

pub fn make_commit(val: u8, parents: Vec<u8>, secs_ago: i64) -> CommitInfo {
    CommitInfo {
        oid: make_oid(val),
        parents: parents.into_iter().map(make_oid).collect(),
        message: format!("commit {val}"),
        author: "test".to_string(),
        time: chrono::Utc::now() - chrono::Duration::seconds(secs_ago),
        source: CommitSource::Local,
    }
}

pub fn make_repo_data(commits: Vec<CommitInfo>, branches: Vec<BranchInfo>) -> RepoData {
    let branch_tips = branches.iter().map(|b| b.tip).collect();
    let head = branches.iter().find(|b| b.is_head).map(|b| b.tip);
    RepoData {
        commits,
        branches,
        tags: vec![],
        head,
        branch_tips,
    }
}
