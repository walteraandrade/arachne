use crate::event::AppEvent;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use tokio::sync::mpsc;

pub fn start_fs_watcher(
    repo_path: &Path,
    pane_idx: usize,
    tx: mpsc::UnboundedSender<AppEvent>,
) -> notify::Result<RecommendedWatcher> {
    let git_dir = repo_path.join(".git");

    let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
        match res {
            Ok(_) => {
                let _ = tx.send(AppEvent::FsChanged(pane_idx));
            }
            Err(e) => {
                eprintln!("fs watcher error: {e}");
            }
        }
    })?;

    let refs_path = git_dir.join("refs");
    let head_path = git_dir.join("HEAD");
    let packed_refs = git_dir.join("packed-refs");

    if refs_path.exists() {
        watcher.watch(&refs_path, RecursiveMode::Recursive)?;
    }
    if head_path.exists() {
        watcher.watch(&head_path, RecursiveMode::NonRecursive)?;
    }
    if packed_refs.exists() {
        let _ = watcher.watch(&packed_refs, RecursiveMode::NonRecursive);
    }

    Ok(watcher)
}
