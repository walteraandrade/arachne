use crate::event::AppEvent;
use std::time::Duration;
use tokio::sync::mpsc;

pub async fn start_github_poller(
    tx: mpsc::UnboundedSender<AppEvent>,
    project_idx: usize,
    interval_secs: u64,
) {
    let secs = interval_secs.max(5);
    let mut interval = tokio::time::interval(Duration::from_secs(secs));
    interval.tick().await;

    loop {
        interval.tick().await;
        if tx.send(AppEvent::GitHubUpdate(project_idx)).is_err() {
            break;
        }
    }
}
