use crate::git::types::{CommitSource, Oid};
use chrono::{DateTime, Utc};

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
}

impl Cell {
    pub fn new(symbol: CellSymbol, color_index: usize) -> Self {
        Self {
            symbol,
            color_index,
        }
    }

    pub fn empty() -> Self {
        Self {
            symbol: CellSymbol::Empty,
            color_index: 0,
        }
    }

    pub fn to_chars(&self) -> &'static str {
        match self.symbol {
            CellSymbol::Commit => "◯ ",
            CellSymbol::Vertical => "│ ",
            CellSymbol::HorizontalLeft => "──",
            CellSymbol::HorizontalRight => "──",
            CellSymbol::MergeDown => "╭─",
            CellSymbol::MergeUp => "╰─",
            CellSymbol::BranchRight => "─╮",
            CellSymbol::BranchLeft => "─╯",
            CellSymbol::Empty => "  ",
        }
    }
}

#[derive(Clone, Debug)]
pub struct GraphRow {
    pub cells: Vec<Cell>,
    pub oid: Oid,
    pub message: String,
    pub author: String,
    pub time_ago: String,
    pub time: DateTime<Utc>,
    pub source: CommitSource,
    pub branch_names: Vec<String>,
    pub tag_names: Vec<String>,
    pub is_head: bool,
}

#[derive(Clone, Debug, Default)]
pub struct LayoutState {
    pub columns: Vec<Option<Oid>>,
}

impl LayoutState {
    pub fn find_column(&self, oid: &Oid) -> Option<usize> {
        self.columns
            .iter()
            .position(|slot| slot.as_ref() == Some(oid))
    }

    pub fn allocate_column(&mut self, oid: Oid) -> usize {
        if let Some(pos) = self.columns.iter().position(|s| s.is_none()) {
            self.columns[pos] = Some(oid);
            pos
        } else {
            self.columns.push(Some(oid));
            self.columns.len() - 1
        }
    }

    pub fn collapse_trailing(&mut self) {
        while self.columns.last() == Some(&None) {
            self.columns.pop();
        }
    }
}
