use crate::git::types::{Oid, RepoData};
use crate::graph::dag::Dag;
use std::collections::HashMap;

pub fn assign_commits_to_branches(
    dag: &Dag,
    repo_data: &RepoData,
    trunk_names: &[String],
) -> HashMap<Oid, usize> {
    let mut map: HashMap<Oid, usize> = HashMap::new();

    for (trunk_idx, trunk_name) in trunk_names.iter().enumerate() {
        let tip = find_branch_tip(repo_data, trunk_name);
        let tip = match tip {
            Some(t) => t,
            None => continue,
        };

        let mut current = tip;
        loop {
            if map.contains_key(&current) {
                break;
            }
            map.insert(current, trunk_idx);

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
        let name = strip_remote_prefix(&branch.name);
        if trunk_names.iter().any(|t| t == name) {
            continue;
        }

        let feature_idx = next_feature_idx;
        next_feature_idx += 1;

        let mut current = branch.tip;
        loop {
            if map.contains_key(&current) {
                break;
            }
            map.insert(current, feature_idx);

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

    map
}

fn find_branch_tip(repo_data: &RepoData, trunk_name: &str) -> Option<Oid> {
    repo_data
        .branches
        .iter()
        .find(|b| {
            let name = strip_remote_prefix(&b.name);
            name == trunk_name
        })
        .map(|b| b.tip)
}

fn strip_remote_prefix(name: &str) -> &str {
    name.find('/').map(|i| &name[i + 1..]).unwrap_or(name)
}
