use crate::git::types::{BranchInfo, Oid, RepoData, TagInfo};
use crate::graph::branch_assign::{assign_commits_to_branches, strip_remote_prefix};
use crate::graph::dag::Dag;
use crate::graph::types::*;
use std::collections::HashMap;

pub fn compute_layout(dag: &Dag, repo_data: &RepoData, trunk_branches: &[String]) -> Vec<GraphRow> {
    let branch_map = build_branch_map(&repo_data.branches);
    let tag_map = build_tag_map(&repo_data.tags);
    let head_oid = repo_data.head.as_ref();
    let commit_branches = assign_commits_to_branches(dag, repo_data, trunk_branches);

    let mut state = LayoutState::new(trunk_branches.len());

    for (trunk_idx, trunk_name) in trunk_branches.iter().enumerate() {
        if let Some(branch) = repo_data.branches.iter().find(|b| {
            let name = strip_remote_prefix(&b.name);
            name == trunk_name
        }) {
            state.columns[trunk_idx] = Some(branch.tip);
        }
    }

    let mut rows = Vec::new();

    for oid in &dag.topo_order {
        let node = match dag.nodes.get(oid) {
            Some(n) => n,
            None => continue,
        };

        let branch_idx = commit_branches.get(oid).copied();
        let is_trunk = branch_idx.is_some_and(|bi| bi < trunk_branches.len());

        let col = state.find_column(oid).unwrap_or_else(|| {
            if let Some(lane) = branch_idx.filter(|_| is_trunk) {
                state.columns[lane] = Some(*oid);
                lane
            } else {
                state.allocate_column_nonreserved(*oid)
            }
        });

        let color = branch_idx.unwrap_or(col + trunk_branches.len());

        let num_cols = state.columns.len();
        let mut cells = Vec::with_capacity(num_cols);

        for i in 0..num_cols {
            if i == col {
                cells.push(Cell::new(CellSymbol::Commit, color));
            } else if state.columns[i].is_some() {
                let ci = commit_branches
                    .get(state.columns[i].as_ref().unwrap())
                    .copied()
                    .unwrap_or(i + trunk_branches.len());
                cells.push(Cell::new(CellSymbol::Vertical, ci));
            } else {
                cells.push(Cell::empty());
            }
        }

        let parents = &node.commit.parents;

        state.columns[col] = None;

        if !parents.is_empty() {
            let first_parent = &parents[0];
            let parent_branch = commit_branches.get(first_parent).copied();
            let parent_is_trunk = parent_branch.is_some_and(|bi| bi < trunk_branches.len());

            if let Some(existing_col) = state.find_column(first_parent) {
                add_merge_cells(&mut cells, col, existing_col, color);
            } else if let Some(lane) = parent_branch.filter(|_| parent_is_trunk) {
                if state.columns[lane].is_none() {
                    state.columns[lane] = Some(*first_parent);
                    if lane != col {
                        extend_cells(&mut cells, lane + 1);
                        add_merge_cells(&mut cells, col, lane, color);
                    }
                } else {
                    state.columns[col] = Some(*first_parent);
                }
            } else {
                state.columns[col] = Some(*first_parent);
            }

            for parent in parents.iter().skip(1) {
                if let Some(pcol) = state.find_column(parent) {
                    add_branch_cells(&mut cells, col, pcol);
                } else {
                    let p_branch = commit_branches.get(parent).copied();
                    let p_is_trunk = p_branch.is_some_and(|bi| bi < trunk_branches.len());

                    let new_col = if let Some(lane) = p_branch.filter(|_| p_is_trunk) {
                        if state.columns[lane].is_none() {
                            state.columns[lane] = Some(*parent);
                            lane
                        } else {
                            state.allocate_column_nonreserved(*parent)
                        }
                    } else {
                        state.allocate_column_nonreserved(*parent)
                    };
                    extend_cells(&mut cells, state.columns.len());
                    add_branch_cells(&mut cells, col, new_col);
                }
            }
        }

        state.collapse_trailing();

        let branch_names = branch_map.get(oid).cloned().unwrap_or_default();
        let tag_names = tag_map.get(oid).cloned().unwrap_or_default();
        let is_head = head_oid == Some(oid);

        rows.push(GraphRow {
            cells,
            oid: *oid,
            message: node.commit.message.clone(),
            author: node.commit.author.clone(),
            time: node.commit.time,
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
            if cell.symbol == CellSymbol::Empty {
                *cell = Cell::new(CellSymbol::HorizontalRight, color);
            }
        }
        if cells[to].symbol == CellSymbol::Empty {
            cells[to] = Cell::new(CellSymbol::BranchLeft, color);
        }
    } else {
        for cell in &mut cells[(lo + 1)..hi] {
            if cell.symbol == CellSymbol::Empty {
                *cell = Cell::new(CellSymbol::HorizontalLeft, color);
            }
        }
        if cells[to].symbol == CellSymbol::Empty {
            cells[to] = Cell::new(CellSymbol::BranchRight, color);
        }
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
        map.entry(b.tip).or_default().push(b.name.clone());
    }
    map
}

fn build_tag_map(tags: &[TagInfo]) -> HashMap<Oid, Vec<String>> {
    let mut map: HashMap<Oid, Vec<String>> = HashMap::new();
    for t in tags {
        map.entry(t.target).or_default().push(t.name.clone());
    }
    map
}

fn duration_bucket(time: &chrono::DateTime<chrono::Utc>) -> (i64, &'static str) {
    let dur = chrono::Utc::now().signed_duration_since(*time);
    if dur.num_seconds() < 60 {
        (dur.num_seconds(), "s")
    } else if dur.num_minutes() < 60 {
        (dur.num_minutes(), "m")
    } else if dur.num_hours() < 24 {
        (dur.num_hours(), "h")
    } else if dur.num_days() < 30 {
        (dur.num_days(), "d")
    } else if dur.num_days() < 365 {
        (dur.num_days() / 30, "mo")
    } else {
        (dur.num_days() / 365, "y")
    }
}

pub fn format_time_ago(time: &chrono::DateTime<chrono::Utc>) -> String {
    let (val, unit) = duration_bucket(time);
    format!("{val}{unit} ago")
}

pub fn format_time_short(time: &chrono::DateTime<chrono::Utc>) -> String {
    let (val, unit) = duration_bucket(time);
    format!("{val}{unit}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::types::*;
    use crate::test_utils::*;

    fn layout_from_commits(commits: Vec<CommitInfo>) -> Vec<GraphRow> {
        let data = make_repo_data(
            commits.clone(),
            vec![BranchInfo {
                name: "main".to_string(),
                tip: commits[0].oid,
                is_head: true,
                source: CommitSource::Local,
            }],
        );
        let dag = Dag::from_repo_data(&data);
        compute_layout(&dag, &data, &[])
    }

    #[test]
    fn test_linear_history() {
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
        let commits = vec![
            make_commit(1, vec![3], 10),
            make_commit(2, vec![4], 15),
            make_commit(3, vec![4], 20),
            make_commit(4, vec![], 30),
        ];
        let data = make_repo_data(
            commits.clone(),
            vec![
                BranchInfo {
                    name: "main".to_string(),
                    tip: commits[0].oid,
                    is_head: true,
                    source: CommitSource::Local,
                },
                BranchInfo {
                    name: "feat".to_string(),
                    tip: commits[1].oid,
                    is_head: false,
                    source: CommitSource::Local,
                },
            ],
        );
        let dag = Dag::from_repo_data(&data);
        let rows = compute_layout(&dag, &data, &[]);
        assert_eq!(rows.len(), 4);
    }

    #[test]
    fn trunk_lane_reservation() {
        let commits = vec![make_commit(1, vec![2], 10), make_commit(2, vec![], 20)];
        let data = make_repo_data(
            commits,
            vec![BranchInfo {
                name: "main".to_string(),
                tip: make_oid(1),
                is_head: true,
                source: CommitSource::Local,
            }],
        );
        let dag = Dag::from_repo_data(&data);
        let trunk = vec!["main".to_string()];
        let rows = compute_layout(&dag, &data, &trunk);

        // trunk commits should be in lane 0
        for row in &rows {
            assert_eq!(row.cells[0].symbol, CellSymbol::Commit);
        }
    }

    #[test]
    fn feature_branch_nonreserved_lane() {
        let commits = vec![
            make_commit(1, vec![3], 10),
            make_commit(2, vec![3], 15),
            make_commit(3, vec![], 20),
        ];
        let data = make_repo_data(
            commits,
            vec![
                BranchInfo {
                    name: "main".to_string(),
                    tip: make_oid(1),
                    is_head: true,
                    source: CommitSource::Local,
                },
                BranchInfo {
                    name: "feat/x".to_string(),
                    tip: make_oid(2),
                    is_head: false,
                    source: CommitSource::Local,
                },
            ],
        );
        let dag = Dag::from_repo_data(&data);
        let trunk = vec!["main".to_string()];
        let rows = compute_layout(&dag, &data, &trunk);

        // feature branch commit should not be in reserved lane 0
        let feat_row = rows.iter().find(|r| r.oid == make_oid(2)).unwrap();
        let commit_col = feat_row
            .cells
            .iter()
            .position(|c| c.symbol == CellSymbol::Commit)
            .unwrap();
        assert!(
            commit_col >= 1,
            "feature should be in lane >= reserved_count"
        );
    }

    #[test]
    fn merge_cell_symbols() {
        // commit 1 merges 2 and 3
        let commits = vec![
            make_commit(1, vec![2, 3], 10),
            make_commit(2, vec![4], 20),
            make_commit(3, vec![4], 25),
            make_commit(4, vec![], 30),
        ];
        let rows = layout_from_commits(commits);

        // merge row should contain branch/merge symbols
        let merge_row = &rows[0];
        let has_merge_sym = merge_row.cells.iter().any(|c| {
            matches!(
                c.symbol,
                CellSymbol::MergeDown
                    | CellSymbol::MergeUp
                    | CellSymbol::BranchRight
                    | CellSymbol::BranchLeft
            )
        });
        assert!(has_merge_sym, "merge commit should have merge/branch cells");
    }
}
