mod app;
mod config;
mod data_source;
mod error;
mod event;
mod git;
mod github;
mod graph;
mod kitty_protocol;
mod project;
mod screen;
mod session;
mod terminal_graphics;
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
use screen::{ConfigScreenState, Screen};
use std::collections::HashSet;
use std::path::PathBuf;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use watcher::fs::FsWatcherHandle;

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

    let graphics_cap = terminal_graphics::detect_graphics_cap();
    let poll_interval = config.poll_interval_secs;
    let is_first_launch = !Config::config_file_exists();
    let mut app = App::new(config, graphics_cap);

    if is_first_launch {
        app.screen = Screen::Config(Box::new(ConfigScreenState::first_launch(&app.config)));
    } else if let Err(e) = app.load_repos() {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }

    // Load session state
    if !is_first_launch {
        session::restore(&mut app);
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
    app.event_tx = Some(tx.clone());

    let mut watchers: Vec<FsWatcherHandle> = Vec::new();
    let mut poller_handles: Vec<JoinHandle<()>> = Vec::new();

    if !is_first_launch {
        start_watchers_and_pollers(&app, &tx, poll_interval, &mut watchers, &mut poller_handles);
    }

    let input_tx = tx.clone();
    let input_handle = tokio::spawn(async move {
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
        app.flush_kitty_if_needed(terminal.backend_mut())?;
        terminal.draw(|f| app.render(f))?;

        let first = if app.has_active_notification() {
            match tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv()).await {
                Ok(Some(e)) => Some(e),
                Ok(None) => break,
                Err(_) => None, // timeout — redraw to dismiss stale toast
            }
        } else {
            match rx.recv().await {
                Some(e) => Some(e),
                None => break,
            }
        };

        let mut fs_changed: HashSet<usize> = HashSet::new();
        let mut config_saved = false;
        if let Some(e) = first {
            process_event(&mut app, e, &mut fs_changed, &tx, &mut config_saved);
        }
        while let Ok(pending) = rx.try_recv() {
            process_event(&mut app, pending, &mut fs_changed, &tx, &mut config_saved);
        }
        for idx in fs_changed {
            app.rebuild_graph(idx);
        }
        if config_saved {
            // Re-start watchers/pollers after config save (first-launch or profile switch)
            for w in &watchers {
                w.debounce_task.abort();
            }
            for h in &poller_handles {
                h.abort();
            }
            watchers.clear();
            poller_handles.clear();
            start_watchers_and_pollers(
                &app,
                &tx,
                app.config.poll_interval_secs,
                &mut watchers,
                &mut poller_handles,
            );
        }

        if app.should_quit {
            break;
        }
    }

    // Save session before exit
    session::save(&app);

    input_handle.abort();
    for w in &watchers {
        w.debounce_task.abort();
    }
    for handle in poller_handles {
        handle.abort();
    }

    app.cleanup_kitty(terminal.backend_mut())?;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

fn start_watchers_and_pollers(
    app: &App,
    tx: &mpsc::UnboundedSender<AppEvent>,
    poll_interval: u64,
    watchers: &mut Vec<FsWatcherHandle>,
    poller_handles: &mut Vec<JoinHandle<()>>,
) {
    for (idx, proj) in app.projects.iter().enumerate() {
        if let Some(ref local) = proj.local_source {
            if let Some(workdir) = local.repo.workdir() {
                let repo_path = workdir.to_path_buf();
                match watcher::fs::start_fs_watcher(&repo_path, idx, tx.clone()) {
                    Ok(w) => watchers.push(w),
                    Err(e) => eprintln!("warn: fs watcher failed for {}: {e}", repo_path.display()),
                }
            }
        }

        if proj.github_client().is_some() {
            let poll_tx = tx.clone();
            let handle = tokio::spawn(async move {
                watcher::poll::start_github_poller(poll_tx, idx, poll_interval).await;
            });
            poller_handles.push(handle);
        }
    }
}

fn process_event(
    app: &mut App,
    event: AppEvent,
    fs_changed: &mut HashSet<usize>,
    tx: &mpsc::UnboundedSender<AppEvent>,
    config_saved: &mut bool,
) {
    match event {
        AppEvent::FsChanged(idx) => {
            fs_changed.insert(idx);
        }
        AppEvent::GitHubUpdate(idx) => {
            if let Some(proj) = app.projects.get(idx) {
                if !proj.github_polling_enabled() {
                    return;
                }
                if let Some(client) = proj.github_client() {
                    let tx = tx.clone();
                    let client = client.clone();
                    tokio::spawn(async move {
                        let result = github::network::fetch_network_detached(&client).await;
                        let event = match result {
                            Ok((branches, commits, rate_limit)) => AppEvent::GitHubResult {
                                project_idx: idx,
                                result: Ok(GitHubData {
                                    rate_limit,
                                    branches,
                                    commits,
                                }),
                            },
                            Err(e) => AppEvent::GitHubResult {
                                project_idx: idx,
                                result: Err(e),
                            },
                        };
                        let _ = tx.send(event);
                    });
                }
            }
        }
        AppEvent::ConfigSaved => {
            *config_saved = true;
        }
        _ => app.handle_event(event),
    }
}
