use crate::event::AppEvent;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use tokio::sync::mpsc;

pub fn start_fs_watcher(
    repo_path: &Path,
    tx: mpsc::UnboundedSender<AppEvent>,
) -> notify::Result<RecommendedWatcher> {
    let git_dir = repo_path.join(".git");

    let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
        if res.is_ok() {
            let _ = tx.send(AppEvent::FsChanged);
        }
    })?;

    let refs_path = git_dir.join("refs");
    let head_path = git_dir.join("HEAD");

    if refs_path.exists() {
        watcher.watch(&refs_path, RecursiveMode::Recursive)?;
    }
    if head_path.exists() {
        watcher.watch(&head_path, RecursiveMode::NonRecursive)?;
    }

    Ok(watcher)
}
