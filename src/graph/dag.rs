use crate::git::types::{CommitInfo, Oid, RepoData};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug)]
pub struct DagNode {
    pub commit: CommitInfo,
    pub children: Vec<Oid>,
}

#[derive(Clone, Debug, Default)]
pub struct Dag {
    pub nodes: HashMap<Oid, DagNode>,
    pub topo_order: Vec<Oid>,
}

impl Dag {
    pub fn from_repo_data(data: &RepoData) -> Self {
        let mut nodes = HashMap::new();
        let mut topo_order = Vec::new();

        for commit in &data.commits {
            nodes.insert(
                commit.oid.clone(),
                DagNode {
                    commit: commit.clone(),
                    children: Vec::new(),
                },
            );
            topo_order.push(commit.oid.clone());
        }

        let oids: HashSet<_> = nodes.keys().cloned().collect();
        for commit in &data.commits {
            for parent_oid in &commit.parents {
                if oids.contains(parent_oid) {
                    if let Some(parent_node) = nodes.get_mut(parent_oid) {
                        parent_node.children.push(commit.oid.clone());
                    }
                }
            }
        }

        Self { nodes, topo_order }
    }

    pub fn merge_remote(&mut self, remote_commits: Vec<CommitInfo>) {
        let existing: HashSet<_> = self.nodes.keys().cloned().collect();

        for commit in remote_commits {
            if existing.contains(&commit.oid) {
                continue;
            }
            self.nodes.insert(
                commit.oid.clone(),
                DagNode {
                    commit: commit.clone(),
                    children: Vec::new(),
                },
            );
        }

        let all_oids: HashSet<_> = self.nodes.keys().cloned().collect();
        let oids_snapshot: Vec<_> = self.nodes.keys().cloned().collect();
        for oid in &oids_snapshot {
            let parents: Vec<_> = self.nodes[oid].commit.parents.clone();
            for parent_oid in &parents {
                if all_oids.contains(parent_oid) {
                    if let Some(parent_node) = self.nodes.get_mut(parent_oid) {
                        if !parent_node.children.contains(oid) {
                            parent_node.children.push(oid.clone());
                        }
                    }
                }
            }
        }

        self.rebuild_topo_order();
    }

    fn rebuild_topo_order(&mut self) {
        let mut sorted: Vec<_> = self.nodes.values().collect();
        sorted.sort_by(|a, b| b.commit.time.cmp(&a.commit.time));
        self.topo_order = sorted.into_iter().map(|n| n.commit.oid.clone()).collect();
    }
}
