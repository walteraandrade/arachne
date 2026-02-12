use crate::git::types::{BranchInfo, Oid, RepoData, TagInfo};
use crate::graph::dag::Dag;
use crate::graph::types::*;
use std::collections::HashMap;

pub fn compute_layout(dag: &Dag, repo_data: &RepoData) -> Vec<GraphRow> {
    let branch_map = build_branch_map(&repo_data.branches);
    let tag_map = build_tag_map(&repo_data.tags);
    let head_oid = repo_data.head.as_ref();
    let mut state = LayoutState::default();
    let mut rows = Vec::new();

    for oid in &dag.topo_order {
        let node = match dag.nodes.get(oid) {
            Some(n) => n,
            None => continue,
        };

        let col = state
            .find_column(oid)
            .unwrap_or_else(|| state.allocate_column(oid.clone()));

        let num_cols = state.columns.len();
        let mut cells = Vec::with_capacity(num_cols);

        for i in 0..num_cols {
            if i == col {
                cells.push(Cell::new(CellSymbol::Commit, col));
            } else if state.columns[i].is_some() {
                cells.push(Cell::new(CellSymbol::Vertical, i));
            } else {
                cells.push(Cell::empty());
            }
        }

        let parents = &node.commit.parents;

        state.columns[col] = None;

        if !parents.is_empty() {
            let first_parent = &parents[0];
            if let Some(existing_col) = state.find_column(first_parent) {
                add_merge_cells(&mut cells, col, existing_col, col);
            } else {
                state.columns[col] = Some(first_parent.clone());
            }

            for parent in parents.iter().skip(1) {
                if state.find_column(parent).is_some() {
                    let pcol = state.find_column(parent).unwrap();
                    add_branch_cells(&mut cells, col, pcol);
                } else {
                    let new_col = state.allocate_column(parent.clone());
                    extend_cells(&mut cells, state.columns.len());
                    add_branch_cells(&mut cells, col, new_col);
                }
            }
        }

        state.collapse_trailing();

        let time_ago = format_time_ago(&node.commit.time);
        let branch_names = branch_map.get(oid).cloned().unwrap_or_default();
        let tag_names = tag_map.get(oid).cloned().unwrap_or_default();
        let is_head = head_oid == Some(oid);

        rows.push(GraphRow {
            cells,
            oid: oid.clone(),
            message: node.commit.message.clone(),
            author: node.commit.author.clone(),
            time_ago,
            source: node.commit.source.clone(),
            branch_names,
            tag_names,
            is_head,
        });
    }

    rows
}

fn extend_cells(cells: &mut Vec<Cell>, target_len: usize) {
    while cells.len() < target_len {
        cells.push(Cell::empty());
    }
}

fn add_merge_cells(cells: &mut Vec<Cell>, from: usize, to: usize, color: usize) {
    if from == to {
        return;
    }
    let (lo, hi) = if from < to { (from, to) } else { (to, from) };

    extend_cells(cells, hi + 1);

    if from < to {
        for cell in &mut cells[(lo + 1)..hi] {
            *cell = Cell::new(CellSymbol::HorizontalRight, color);
        }
        cells[to] = Cell::new(CellSymbol::BranchLeft, color);
    } else {
        for cell in &mut cells[(lo + 1)..hi] {
            *cell = Cell::new(CellSymbol::HorizontalLeft, color);
        }
        cells[to] = Cell::new(CellSymbol::BranchRight, color);
    }
}

fn add_branch_cells(cells: &mut Vec<Cell>, from: usize, to: usize) {
    if from == to {
        return;
    }
    let (lo, hi) = if from < to { (from, to) } else { (to, from) };

    extend_cells(cells, hi + 1);

    if from < to {
        for cell in &mut cells[(lo + 1)..hi] {
            if cell.symbol == CellSymbol::Empty {
                *cell = Cell::new(CellSymbol::HorizontalRight, to);
            }
        }
        if cells[to].symbol == CellSymbol::Empty || cells[to].symbol == CellSymbol::Vertical {
            cells[to] = Cell::new(CellSymbol::MergeDown, to);
        }
    } else {
        for cell in &mut cells[(lo + 1)..hi] {
            if cell.symbol == CellSymbol::Empty {
                *cell = Cell::new(CellSymbol::HorizontalLeft, to);
            }
        }
        if cells[to].symbol == CellSymbol::Empty || cells[to].symbol == CellSymbol::Vertical {
            cells[to] = Cell::new(CellSymbol::MergeUp, to);
        }
    }
}

fn build_branch_map(branches: &[BranchInfo]) -> HashMap<Oid, Vec<String>> {
    let mut map: HashMap<Oid, Vec<String>> = HashMap::new();
    for b in branches {
        map.entry(b.tip.clone()).or_default().push(b.name.clone());
    }
    map
}

fn build_tag_map(tags: &[TagInfo]) -> HashMap<Oid, Vec<String>> {
    let mut map: HashMap<Oid, Vec<String>> = HashMap::new();
    for t in tags {
        map.entry(t.target.clone())
            .or_default()
            .push(t.name.clone());
    }
    map
}

fn format_time_ago(time: &chrono::DateTime<chrono::Utc>) -> String {
    let now = chrono::Utc::now();
    let dur = now.signed_duration_since(*time);

    if dur.num_seconds() < 60 {
        return format!("{}s ago", dur.num_seconds());
    }
    if dur.num_minutes() < 60 {
        return format!("{}m ago", dur.num_minutes());
    }
    if dur.num_hours() < 24 {
        return format!("{}h ago", dur.num_hours());
    }
    if dur.num_days() < 30 {
        return format!("{}d ago", dur.num_days());
    }
    if dur.num_days() < 365 {
        return format!("{}mo ago", dur.num_days() / 30);
    }
    format!("{}y ago", dur.num_days() / 365)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::types::*;

    fn make_oid(val: u8) -> Oid {
        let mut bytes = [0u8; 20];
        bytes[0] = val;
        Oid(bytes)
    }

    fn make_commit(val: u8, parents: Vec<u8>, secs_ago: i64) -> CommitInfo {
        CommitInfo {
            oid: make_oid(val),
            parents: parents.into_iter().map(make_oid).collect(),
            message: format!("commit {val}"),
            author: "test".to_string(),
            time: chrono::Utc::now() - chrono::Duration::seconds(secs_ago),
            source: CommitSource::Local,
        }
    }

    fn layout_from_commits(commits: Vec<CommitInfo>) -> Vec<GraphRow> {
        let data = RepoData {
            commits: commits.clone(),
            branches: vec![BranchInfo {
                name: "main".to_string(),
                tip: commits[0].oid.clone(),
                is_head: true,
                source: CommitSource::Local,
            }],
            tags: vec![],
            head: Some(commits[0].oid.clone()),
            branch_tips: [commits[0].oid.clone()].into_iter().collect(),
        };
        let dag = Dag::from_repo_data(&data);
        compute_layout(&dag, &data)
    }

    #[test]
    fn test_linear_history() {
        // A → B → C (linear, no branches)
        let commits = vec![
            make_commit(1, vec![2], 10),
            make_commit(2, vec![3], 20),
            make_commit(3, vec![], 30),
        ];
        let rows = layout_from_commits(commits);

        assert_eq!(rows.len(), 3);
        for row in &rows {
            assert_eq!(row.cells.len(), 1);
            assert_eq!(row.cells[0].symbol, CellSymbol::Commit);
        }
    }

    #[test]
    fn test_branch_and_merge() {
        //  1 (merge of 2 and 3)
        //  |\
        //  2 3
        //  |/
        //  4
        let commits = vec![
            make_commit(1, vec![2, 3], 10),
            make_commit(2, vec![4], 20),
            make_commit(3, vec![4], 25),
            make_commit(4, vec![], 30),
        ];
        let rows = layout_from_commits(commits);

        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].cells[0].symbol, CellSymbol::Commit);
        assert!(rows[0].cells.len() >= 2);
    }

    #[test]
    fn test_octopus_merge() {
        //  1 (merge of 2, 3, 4)
        //  |\|
        //  2 3 4
        //  |/|/
        //  5
        let commits = vec![
            make_commit(1, vec![2, 3, 4], 10),
            make_commit(2, vec![5], 20),
            make_commit(3, vec![5], 25),
            make_commit(4, vec![5], 28),
            make_commit(5, vec![], 30),
        ];
        let rows = layout_from_commits(commits);

        assert_eq!(rows.len(), 5);
        assert!(rows[0].cells.len() >= 3);
    }

    #[test]
    fn test_single_commit() {
        let commits = vec![make_commit(1, vec![], 10)];
        let rows = layout_from_commits(commits);

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].cells.len(), 1);
        assert_eq!(rows[0].cells[0].symbol, CellSymbol::Commit);
    }

    #[test]
    fn test_divergent_branches() {
        //  1   2  (two branch tips)
        //  |   |
        //  3   |
        //   \ /
        //    4
        let commits = vec![
            make_commit(1, vec![3], 10),
            make_commit(2, vec![4], 15),
            make_commit(3, vec![4], 20),
            make_commit(4, vec![], 30),
        ];
        let data = RepoData {
            commits: commits.clone(),
            branches: vec![
                BranchInfo {
                    name: "main".to_string(),
                    tip: commits[0].oid.clone(),
                    is_head: true,
                    source: CommitSource::Local,
                },
                BranchInfo {
                    name: "feat".to_string(),
                    tip: commits[1].oid.clone(),
                    is_head: false,
                    source: CommitSource::Local,
                },
            ],
            tags: vec![],
            head: Some(commits[0].oid.clone()),
            branch_tips: [commits[0].oid.clone(), commits[1].oid.clone()]
                .into_iter()
                .collect(),
        };
        let dag = Dag::from_repo_data(&data);
        let rows = compute_layout(&dag, &data);

        assert_eq!(rows.len(), 4);
    }
}
