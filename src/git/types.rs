use chrono::{DateTime, Utc};
use std::collections::HashSet;
use std::fmt;
use std::hash::{Hash, Hasher};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Oid([u8; 20]);

impl Oid {
    pub fn from_git2(oid: git2::Oid) -> Self {
        let mut bytes = [0u8; 20];
        bytes.copy_from_slice(oid.as_bytes());
        Self(bytes)
    }

    pub fn zero() -> Self {
        Self([0u8; 20])
    }

    pub fn from_bytes(bytes: [u8; 20]) -> Self {
        Self(bytes)
    }
}

impl Hash for Oid {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(u64::from_le_bytes(self.0[..8].try_into().unwrap()));
    }
}

impl fmt::Display for Oid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for b in &self.0 {
            write!(f, "{b:02x}")?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommitSource {
    Local,
    Remote(String),
    Fork(String),
}

#[derive(Clone, Debug)]
pub struct CommitInfo {
    pub oid: Oid,
    pub parents: Vec<Oid>,
    pub message: String,
    pub author: String,
    pub time: DateTime<Utc>,
    pub source: CommitSource,
}

#[derive(Clone, Debug)]
pub struct BranchInfo {
    pub name: String,
    pub tip: Oid,
    pub is_head: bool,
    pub source: CommitSource,
}

#[derive(Clone, Debug)]
pub struct TagInfo {
    pub name: String,
    pub target: Oid,
}

#[derive(Clone, Debug, Default)]
pub struct RepoData {
    pub commits: Vec<CommitInfo>,
    pub branches: Vec<BranchInfo>,
    pub tags: Vec<TagInfo>,
    pub head: Option<Oid>,
    pub branch_tips: HashSet<Oid>,
}
