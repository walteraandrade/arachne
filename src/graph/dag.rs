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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;

    fn simple_repo_data(commits: Vec<CommitInfo>) -> RepoData {
        let tip = commits[0].oid;
        make_repo_data(
            commits,
            vec![crate::git::types::BranchInfo {
                name: "main".to_string(),
                tip,
                is_head: true,
                source: crate::git::types::CommitSource::Local,
            }],
        )
    }

    #[test]
    fn linear_chain() {
        let commits = vec![
            make_commit(1, vec![2], 10),
            make_commit(2, vec![3], 20),
            make_commit(3, vec![], 30),
        ];
        let dag = Dag::from_repo_data(&simple_repo_data(commits));

        assert_eq!(dag.topo_order.len(), 3);
        assert_eq!(dag.topo_order[0], make_oid(1));
        assert_eq!(dag.topo_order[1], make_oid(2));
        assert_eq!(dag.topo_order[2], make_oid(3));

        assert_eq!(dag.nodes[&make_oid(3)].children, vec![make_oid(2)]);
        assert_eq!(dag.nodes[&make_oid(2)].children, vec![make_oid(1)]);
        assert!(dag.nodes[&make_oid(1)].children.is_empty());
    }

    #[test]
    fn diamond_merge() {
        // 1 merges 2+3, both parent 4
        let commits = vec![
            make_commit(1, vec![2, 3], 10),
            make_commit(2, vec![4], 20),
            make_commit(3, vec![4], 25),
            make_commit(4, vec![], 30),
        ];
        let dag = Dag::from_repo_data(&simple_repo_data(commits));

        assert_eq!(dag.topo_order.len(), 4);
        assert_eq!(dag.topo_order[0], make_oid(1));

        let node4 = &dag.nodes[&make_oid(4)];
        assert_eq!(node4.children.len(), 2);
        assert!(node4.children.contains(&make_oid(2)));
        assert!(node4.children.contains(&make_oid(3)));
    }

    #[test]
    fn empty_repo() {
        let data = make_repo_data(vec![], vec![]);
        let dag = Dag::from_repo_data(&data);
        assert!(dag.nodes.is_empty());
        assert!(dag.topo_order.is_empty());
    }

    #[test]
    fn merge_remote_adds_new_skips_dupes() {
        let commits = vec![make_commit(1, vec![2], 10), make_commit(2, vec![], 20)];
        let mut dag = Dag::from_repo_data(&simple_repo_data(commits));
        assert_eq!(dag.nodes.len(), 2);

        let remote = vec![
            make_commit(2, vec![], 20),  // dupe
            make_commit(3, vec![2], 15), // new
        ];
        dag.merge_remote(remote);

        assert_eq!(dag.nodes.len(), 3);
        assert!(dag.nodes.contains_key(&make_oid(3)));
        // new commit should be wired as child of 2
        assert!(dag.nodes[&make_oid(2)].children.contains(&make_oid(3)));
    }

    #[test]
    fn kahns_sort_time_tiebreaking() {
        // Two tips with no relationship — newer should come first
        let commits = vec![
            make_commit(1, vec![], 10), // newer
            make_commit(2, vec![], 20), // older
        ];
        let data = make_repo_data(
            commits,
            vec![
                crate::git::types::BranchInfo {
                    name: "a".to_string(),
                    tip: make_oid(1),
                    is_head: true,
                    source: crate::git::types::CommitSource::Local,
                },
                crate::git::types::BranchInfo {
                    name: "b".to_string(),
                    tip: make_oid(2),
                    is_head: false,
                    source: crate::git::types::CommitSource::Local,
                },
            ],
        );
        let dag = Dag::from_repo_data(&data);
        // Newer (1) should come before older (2) due to time tiebreaking
        assert_eq!(dag.topo_order[0], make_oid(1));
        assert_eq!(dag.topo_order[1], make_oid(2));
    }
}
