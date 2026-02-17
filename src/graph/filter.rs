use crate::git::types::{Oid, RepoData};
use std::collections::{HashMap, HashSet, VecDeque};

pub fn filter_by_author(data: &mut RepoData, author_query: &str) {
    let query = author_query.to_lowercase();
    let commit_map: HashMap<_, _> = data.commits.iter().map(|c| (c.oid, c)).collect();

    let matching: HashSet<Oid> = data
        .commits
        .iter()
        .filter(|c| c.author.to_lowercase().contains(&query))
        .map(|c| c.oid)
        .collect();

    let matching_branches: Vec<_> = data
        .branches
        .iter()
        .filter(|branch| {
            let mut visited = HashSet::new();
            let mut queue = VecDeque::new();
            queue.push_back(branch.tip);
            while let Some(oid) = queue.pop_front() {
                if !visited.insert(oid) {
                    continue;
                }
                if matching.contains(&oid) {
                    return true;
                }
                if let Some(commit) = commit_map.get(&oid) {
                    queue.extend(commit.parents.iter().copied());
                }
            }
            false
        })
        .cloned()
        .collect();

    let mut skip_cache: HashMap<Oid, Vec<Oid>> = HashMap::new();
    for commit in data.commits.iter().rev() {
        if matching.contains(&commit.oid) {
            skip_cache.insert(commit.oid, vec![commit.oid]);
        } else {
            let mut ancestors = Vec::new();
            let mut seen = HashSet::new();
            for &parent in &commit.parents {
                if let Some(parent_ancestors) = skip_cache.get(&parent) {
                    for &a in parent_ancestors {
                        if seen.insert(a) {
                            ancestors.push(a);
                        }
                    }
                }
            }
            skip_cache.insert(commit.oid, ancestors);
        }
    }

    for commit in &mut data.commits {
        if !matching.contains(&commit.oid) {
            continue;
        }
        let mut new_parents = Vec::new();
        let mut seen = HashSet::new();
        for &parent in &commit.parents {
            if matching.contains(&parent) {
                if seen.insert(parent) {
                    new_parents.push(parent);
                }
            } else if let Some(ancestors) = skip_cache.get(&parent) {
                for &a in ancestors {
                    if seen.insert(a) {
                        new_parents.push(a);
                    }
                }
            }
        }
        commit.parents = new_parents;
    }

    data.commits.retain(|c| matching.contains(&c.oid));

    data.branches = matching_branches
        .into_iter()
        .filter_map(|mut branch| {
            if matching.contains(&branch.tip) {
                Some(branch)
            } else {
                let ancestors = skip_cache.get(&branch.tip)?;
                branch.tip = *ancestors.first()?;
                Some(branch)
            }
        })
        .collect();

    data.branch_tips = data.branches.iter().map(|b| b.tip).collect();
}
