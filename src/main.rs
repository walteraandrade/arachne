mod app;
mod config;
mod error;
mod event;
mod git;
mod github;
mod graph;
#[cfg(test)]
mod test_utils;
mod ui;
mod watcher;

use app::App;
use clap::Parser;
use config::Config;
use crossterm::{
    event::{Event, EventStream, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use event::{AppEvent, GitHubData};
use futures::StreamExt;
use std::collections::HashSet;
use watcher::fs::FsWatcherHandle;
use std::path::PathBuf;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

#[derive(Parser)]
#[command(name = "arachne", about = "TUI git network graph viewer")]
struct Cli {
    #[arg(long, short, help = "Path to git repository")]
    repo: Option<PathBuf>,
}

// git2::Repository is !Send — current_thread avoids the need to shuffle it across threads
#[tokio::main(flavor = "current_thread")]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let config = Config::load(cli.repo);

    let poll_interval = config.poll_interval_secs;
    let mut app = App::new(config);
    if let Err(e) = app.load_repos() {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }

    // Install panic hook before entering raw mode so terminal is restored on panic
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
        default_hook(info);
    }));

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    let (tx, mut rx) = mpsc::unbounded_channel::<AppEvent>();

    let mut watchers: Vec<FsWatcherHandle> = Vec::new();
    let mut poller_handles: Vec<JoinHandle<()>> = Vec::new();

    for (idx, pane) in app.panes.iter().enumerate() {
        if let Some(ref r) = pane.repo {
            if let Some(workdir) = r.workdir() {
                let repo_path = workdir.to_path_buf();
                if let Ok(w) = watcher::fs::start_fs_watcher(&repo_path, idx, tx.clone()) {
                    watchers.push(w);
                }
            }
        }

        if pane.github_client.is_some() {
            let poll_tx = tx.clone();
            let handle = tokio::spawn(async move {
                watcher::poll::start_github_poller(poll_tx, idx, poll_interval).await;
            });
            poller_handles.push(handle);
        }
    }

    let input_tx = tx.clone();
    tokio::spawn(async move {
        let mut reader = EventStream::new();
        while let Some(Ok(event)) = reader.next().await {
            let app_event = match event {
                Event::Key(key) if key.kind == KeyEventKind::Press => Some(AppEvent::Key(key)),
                Event::Resize(_, _) => Some(AppEvent::Resize),
                _ => None,
            };
            if let Some(e) = app_event {
                if input_tx.send(e).is_err() {
                    break;
                }
            }
        }
    });

    loop {
        terminal.draw(|f| app.render(f))?;

        let first = match rx.recv().await {
            Some(e) => e,
            None => break,
        };

        let mut fs_changed: HashSet<usize> = HashSet::new();
        process_event(&mut app, first, &mut fs_changed, &tx);
        while let Ok(pending) = rx.try_recv() {
            process_event(&mut app, pending, &mut fs_changed, &tx);
        }
        for idx in fs_changed {
            app.rebuild_graph(idx);
        }

        if app.should_quit {
            break;
        }
    }

    for w in &watchers {
        w.debounce_task.abort();
    }
    for handle in poller_handles {
        handle.abort();
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

fn process_event(
    app: &mut App,
    event: AppEvent,
    fs_changed: &mut HashSet<usize>,
    tx: &mpsc::UnboundedSender<AppEvent>,
) {
    match event {
        AppEvent::FsChanged(idx) => {
            fs_changed.insert(idx);
        }
        AppEvent::GitHubUpdate(idx) => {
            if let Some(pane) = app.panes.get(idx) {
                if let Some(ref client) = pane.github_client {
                    let tx = tx.clone();
                    let client = client.clone();
                    tokio::spawn(async move {
                        let result = github::network::fetch_network_detached(&client).await;
                        let event = match result {
                            Ok((branches, commits, rate_limit)) => {
                                AppEvent::GitHubResult {
                                    pane_idx: idx,
                                    result: Ok(GitHubData { rate_limit, branches, commits }),
                                }
                            }
                            Err(e) => {
                                AppEvent::GitHubResult {
                                    pane_idx: idx,
                                    result: Err(e),
                                }
                            }
                        };
                        let _ = tx.send(event);
                    });
                }
            }
        }
        _ => app.handle_event(event),
    }
}
