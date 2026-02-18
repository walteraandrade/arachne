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
        let bytes: [u8; 8] = [
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5], self.0[6], self.0[7],
        ];
        state.write_u64(u64::from_le_bytes(bytes));
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
    pub time: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Default)]
pub struct RepoData {
    pub commits: Vec<CommitInfo>,
    pub branches: Vec<BranchInfo>,
    pub tags: Vec<TagInfo>,
    pub head: Option<Oid>,
    pub branch_tips: HashSet<Oid>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::hash_map::DefaultHasher;

    #[test]
    fn from_bytes_roundtrip() {
        let mut bytes = [0u8; 20];
        bytes[0] = 0xab;
        bytes[19] = 0xcd;
        let oid = Oid::from_bytes(bytes);
        let display = format!("{oid}");
        assert!(display.starts_with("ab"));
        assert!(display.ends_with("cd"));
        assert_eq!(display.len(), 40);
    }

    #[test]
    fn zero_oid() {
        let z = Oid::zero();
        assert_eq!(format!("{z}"), "0000000000000000000000000000000000000000");
    }

    #[test]
    fn hash_consistency() {
        let a = Oid::from_bytes([1; 20]);
        let b = Oid::from_bytes([1; 20]);
        let c = Oid::from_bytes([2; 20]);

        let hash = |o: &Oid| {
            let mut h = DefaultHasher::new();
            o.hash(&mut h);
            h.finish()
        };
        assert_eq!(hash(&a), hash(&b));
        assert_ne!(hash(&a), hash(&c));
    }

    #[test]
    fn ord() {
        let a = Oid::from_bytes([0; 20]);
        let b = Oid::from_bytes([1; 20]);
        assert!(a < b);
        assert_eq!(a, a);
    }
}
