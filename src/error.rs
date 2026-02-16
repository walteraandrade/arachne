use thiserror::Error;

#[derive(Error, Debug)]
pub enum ArachneError {
    #[error("git error: {0}")]
    Git(#[from] git2::Error),

    #[error("github error: {0}")]
    GitHub(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("watcher error: {0}")]
    Watcher(#[from] notify::Error),

    #[error("not a git repository: {0}")]
    NotARepo(String),
}

pub type Result<T> = std::result::Result<T, ArachneError>;
