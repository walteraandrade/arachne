use crate::event::AppEvent;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

pub struct FsWatcherHandle {
    pub _watcher: RecommendedWatcher,
    pub debounce_task: JoinHandle<()>,
}

pub fn start_fs_watcher(
    repo_path: &Path,
    pane_idx: usize,
    tx: mpsc::UnboundedSender<AppEvent>,
) -> notify::Result<FsWatcherHandle> {
    let git_dir = repo_path.join(".git");

    let (raw_tx, mut raw_rx) = mpsc::unbounded_channel::<()>();

    let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
        if res.is_ok() {
            let _ = raw_tx.send(());
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

    let debounce_task = tokio::spawn(async move {
        while raw_rx.recv().await.is_some() {
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            while raw_rx.try_recv().is_ok() {}
            let _ = tx.send(AppEvent::FsChanged(pane_idx));
        }
    });

    Ok(FsWatcherHandle {
        _watcher: watcher,
        debounce_task,
    })
}
