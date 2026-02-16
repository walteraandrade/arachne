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

        for commit in &data.commits {
            nodes.insert(
                commit.oid,
                DagNode {
                    commit: commit.clone(),
                    children: Vec::new(),
                },
            );
        }

        let oids: HashSet<_> = nodes.keys().copied().collect();
        for commit in &data.commits {
            for parent_oid in &commit.parents {
                if oids.contains(parent_oid) {
                    if let Some(parent_node) = nodes.get_mut(parent_oid) {
                        parent_node.children.push(commit.oid);
                    }
                }
            }
        }

        let topo_order = kahns_topo_sort(&nodes);
        Self { nodes, topo_order }
    }

    pub fn merge_remote(&mut self, remote_commits: Vec<CommitInfo>) {
        let mut newly_inserted = Vec::new();

        for commit in remote_commits {
            if self.nodes.contains_key(&commit.oid) {
                continue;
            }
            let oid = commit.oid;
            self.nodes.insert(
                oid,
                DagNode {
                    commit,
                    children: Vec::new(),
                },
            );
            newly_inserted.push(oid);
        }

        let all_oids: HashSet<_> = self.nodes.keys().copied().collect();
        for oid in &newly_inserted {
            let parents: Vec<_> = self.nodes[oid].commit.parents.clone();
            for parent_oid in &parents {
                if all_oids.contains(parent_oid) {
                    if let Some(parent_node) = self.nodes.get_mut(parent_oid) {
                        if !parent_node.children.contains(oid) {
                            parent_node.children.push(*oid);
                        }
                    }
                }
            }
        }

        self.topo_order = kahns_topo_sort(&self.nodes);
    }
}

fn kahns_topo_sort(nodes: &HashMap<Oid, DagNode>) -> Vec<Oid> {
    // In-degree: count how many children point to each node (children → parent edges)
    // We want newest-first, so "edges" go from child→parent.
    // In-degree = number of children (nodes that have this as parent).
    let mut in_degree: HashMap<Oid, usize> = HashMap::new();
    for oid in nodes.keys() {
        in_degree.entry(*oid).or_insert(0);
    }
    for node in nodes.values() {
        for parent_oid in &node.commit.parents {
            if nodes.contains_key(parent_oid) {
                *in_degree.entry(*parent_oid).or_insert(0) += 1;
            }
        }
    }

    // Start from nodes with 0 in-degree (no children point to them = tips/leaves)
    // Use a BinaryHeap to break ties by time descending
    use std::collections::BinaryHeap;

    let mut heap: BinaryHeap<(chrono::DateTime<chrono::Utc>, Oid)> = BinaryHeap::new();
    for (&oid, &deg) in &in_degree {
        if deg == 0 {
            let time = nodes[&oid].commit.time;
            heap.push((time, oid));
        }
    }

    let mut result = Vec::with_capacity(nodes.len());
    while let Some((_, oid)) = heap.pop() {
        result.push(oid);
        let parents: Vec<_> = nodes[&oid].commit.parents.clone();
        for parent_oid in parents {
            if let Some(deg) = in_degree.get_mut(&parent_oid) {
                *deg -= 1;
                if *deg == 0 {
                    if let Some(pnode) = nodes.get(&parent_oid) {
                        heap.push((pnode.commit.time, parent_oid));
                    }
                }
            }
        }
    }

    result
}
