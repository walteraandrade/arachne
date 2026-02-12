use crossterm::event::KeyEvent;

#[derive(Debug)]
pub enum AppEvent {
    Key(KeyEvent),
    Resize(u16, u16),
    FsChanged(usize),
    GitHubUpdate(usize),
    Tick,
    Error(String),
}
