use crate::git::types::{CommitSource, Oid, RepoData};
use crate::graph::dag::Dag;
use std::collections::HashMap;

pub struct BranchAssignment {
    pub commit_to_branch: HashMap<Oid, usize>,
    pub index_to_name: HashMap<usize, String>,
    pub trunk_count: usize,
}

pub fn assign_branches(
    dag: &Dag,
    repo_data: &RepoData,
    trunk_names: &[String],
) -> BranchAssignment {
    let mut commit_to_branch: HashMap<Oid, usize> = HashMap::new();
    let mut index_to_name: HashMap<usize, String> = HashMap::new();

    for (trunk_idx, trunk_name) in trunk_names.iter().enumerate() {
        let tip = find_branch_tip(repo_data, trunk_name);
        let tip = match tip {
            Some(t) => t,
            None => continue,
        };

        index_to_name.insert(trunk_idx, trunk_name.clone());

        let mut current = tip;
        loop {
            if commit_to_branch.contains_key(&current) {
                break;
            }
            commit_to_branch.insert(current, trunk_idx);

            let node = match dag.nodes.get(&current) {
                Some(n) => n,
                None => break,
            };
            match node.commit.parents.first() {
                Some(parent) => current = *parent,
                None => break,
            }
        }
    }

    let mut next_feature_idx = trunk_names.len();
    for branch in &repo_data.branches {
        if is_trunk_match(&branch.name, &branch.source, trunk_names) {
            continue;
        }

        let feature_idx = next_feature_idx;
        next_feature_idx += 1;
        index_to_name.insert(feature_idx, branch.name.clone());

        let mut current = branch.tip;
        loop {
            if commit_to_branch.contains_key(&current) {
                break;
            }
            commit_to_branch.insert(current, feature_idx);

            let node = match dag.nodes.get(&current) {
                Some(n) => n,
                None => break,
            };
            match node.commit.parents.first() {
                Some(parent) => current = *parent,
                None => break,
            }
        }
    }

    BranchAssignment {
        commit_to_branch,
        index_to_name,
        trunk_count: trunk_names.len(),
    }
}

fn is_trunk_match(name: &str, source: &CommitSource, trunk_names: &[String]) -> bool {
    let compare_name = match source {
        CommitSource::Local => name,
        _ => strip_remote_prefix(name),
    };
    trunk_names.iter().any(|t| t == compare_name)
}

fn find_branch_tip(repo_data: &RepoData, trunk_name: &str) -> Option<Oid> {
    repo_data
        .branches
        .iter()
        .find(|b| is_trunk_match(&b.name, &b.source, &[trunk_name.to_string()]))
        .map(|b| b.tip)
}

pub fn strip_remote_prefix(name: &str) -> &str {
    name.find('/').map(|i| &name[i + 1..]).unwrap_or(name)
}
