mod app;
mod config;
mod error;
mod event;
mod git;
mod github;
mod graph;
mod ui;
mod watcher;

use app::App;
use clap::Parser;
use config::Config;
use crossterm::{
    event::{self as ct_event, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use event::AppEvent;
use notify::RecommendedWatcher;
use std::path::PathBuf;
use tokio::sync::mpsc;

#[derive(Parser)]
#[command(name = "arachne", about = "TUI git network graph viewer")]
struct Cli {
    #[arg(long, short, help = "Path to git repository")]
    repo: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> color_eyre_fallback::Result<()> {
    let cli = Cli::parse();
    let config = Config::load(cli.repo);

    let mut app = App::new(config.clone());
    if let Err(e) = app.load_repos() {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    let (tx, mut rx) = mpsc::unbounded_channel::<AppEvent>();

    // Spawn per-repo FS watchers and GitHub pollers
    let mut _watchers: Vec<RecommendedWatcher> = Vec::new();
    for (idx, pane) in app.panes.iter().enumerate() {
        if let Some(ref r) = pane.repo {
            if let Some(workdir) = r.workdir() {
                let repo_path = workdir.to_path_buf();
                if let Ok(w) = watcher::fs::start_fs_watcher(&repo_path, idx, tx.clone()) {
                    _watchers.push(w);
                }
            }
        }

        if pane.github_client.is_some() {
            let poll_tx = tx.clone();
            let poll_interval = config.poll_interval_secs;
            tokio::spawn(async move {
                watcher::poll::start_github_poller(poll_tx, idx, poll_interval).await;
            });
        }
    }

    let input_tx = tx.clone();
    tokio::spawn(async move {
        loop {
            if ct_event::poll(std::time::Duration::from_millis(50)).unwrap_or(false) {
                if let Ok(event) = ct_event::read() {
                    let app_event = match event {
                        Event::Key(key) if key.kind == KeyEventKind::Press => {
                            Some(AppEvent::Key(key))
                        }
                        Event::Resize(w, h) => Some(AppEvent::Resize(w, h)),
                        _ => None,
                    };
                    if let Some(e) = app_event {
                        if input_tx.send(e).is_err() {
                            break;
                        }
                    }
                }
            } else {
                let _ = input_tx.send(AppEvent::Tick);
            }
        }
    });

    loop {
        terminal.draw(|f| app.render(f))?;

        if let Some(event) = rx.recv().await {
            match &event {
                AppEvent::GitHubUpdate(idx) => {
                    let idx = *idx;
                    app.fetch_github(idx).await;
                }
                _ => app.handle_event(event),
            }
        }

        if app.should_quit {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

mod color_eyre_fallback {
    pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
}
