use crate::git::types::{CommitSource, Oid};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CellSymbol {
    Commit,
    Vertical,
    HorizontalLeft,
    HorizontalRight,
    MergeDown,
    MergeUp,
    BranchRight,
    BranchLeft,
    Empty,
}

#[derive(Clone, Debug)]
pub struct Cell {
    pub symbol: CellSymbol,
    pub color_index: usize,
    pub trunk_index: Option<usize>,
}

impl Cell {
    pub fn new(symbol: CellSymbol, color_index: usize) -> Self {
        Self {
            symbol,
            color_index,
            trunk_index: None,
        }
    }

    pub fn empty() -> Self {
        Self {
            symbol: CellSymbol::Empty,
            color_index: 0,
            trunk_index: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Edge {
    pub from_lane: usize,
    pub to_lane: usize,
    pub kind: EdgeKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EdgeKind {
    MergeToParent { color_index: usize },
    BranchToParent,
}

#[derive(Clone, Debug)]
pub struct LaneOccupant {
    pub lane: usize,
    pub color_index: usize,
    pub trunk_index: Option<usize>,
}

#[derive(Clone, Debug)]
pub struct RowLayout {
    pub commit_lane: usize,
    pub commit_color: usize,
    pub trunk_index: Option<usize>,
    pub edges: Vec<Edge>,
    pub passthrough_lanes: Vec<LaneOccupant>,
    pub lane_branches: Vec<Option<usize>>,
}

#[derive(Clone, Debug)]
pub struct RowMeta {
    pub oid: Oid,
    pub message: String,
    pub author: String,
    pub time: DateTime<Utc>,
    pub source: CommitSource,
    pub branch_names: Vec<String>,
    pub tag_names: Vec<String>,
    pub is_head: bool,
    pub branch_index: Option<usize>,
    pub is_merge: bool,
    pub is_fork_point: bool,
}

#[derive(Clone, Debug)]
pub struct GraphRow {
    pub layout: RowLayout,
    pub meta: RowMeta,
    pub cells: Vec<Cell>,
}

#[derive(Clone, Debug)]
pub struct LayoutResult {
    pub rows: Vec<GraphRow>,
    pub branch_index_to_name: HashMap<usize, String>,
    pub trunk_count: usize,
    pub max_lanes: usize,
}

pub fn num_lanes_for_layout(layout: &RowLayout) -> usize {
    layout
        .passthrough_lanes
        .iter()
        .map(|p| p.lane + 1)
        .chain(layout.edges.iter().map(|e| e.from_lane.max(e.to_lane) + 1))
        .max()
        .unwrap_or(0)
        .max(layout.commit_lane + 1)
}

pub const MAX_LANES: usize = 64;

#[derive(Clone, Debug, Default)]
pub struct LayoutState {
    pub columns: Vec<Option<Oid>>,
    pub reserved_count: usize,
}

impl LayoutState {
    pub fn new(reserved: usize) -> Self {
        let mut columns = Vec::with_capacity(reserved);
        columns.resize_with(reserved, || None);
        Self {
            columns,
            reserved_count: reserved,
        }
    }

    pub fn find_column(&self, oid: &Oid) -> Option<usize> {
        self.columns
            .iter()
            .position(|slot| slot.as_ref() == Some(oid))
    }

    pub fn allocate_column_nonreserved(&mut self, oid: Oid) -> usize {
        if let Some(pos) = self
            .columns
            .iter()
            .skip(self.reserved_count)
            .position(|s| s.is_none())
        {
            let idx = pos + self.reserved_count;
            self.columns[idx] = Some(oid);
            idx
        } else if self.columns.len() >= MAX_LANES {
            eprintln!("warning: MAX_LANES exceeded; reusing last column");
            let last = self.columns.len() - 1;
            self.columns[last] = Some(oid);
            last
        } else {
            self.columns.push(Some(oid));
            self.columns.len() - 1
        }
    }

    pub fn collapse_trailing(&mut self) {
        while self.columns.len() > self.reserved_count && self.columns.last() == Some(&None) {
            self.columns.pop();
        }
    }
}
