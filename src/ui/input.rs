use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    ScrollUp,
    ScrollDown,
    ScrollLeft,
    ScrollRight,
    PanelLeft,
    PanelRight,
    NextPane,
    PrevPane,
    Select,
    ToggleForks,
    Filter,
    AuthorFilter,
    FilterChar(char),
    FilterBackspace,
    FilterConfirm,
    FilterCancel,
    Refresh,
    Help,
    ClosePopup,
    Quit,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterMode {
    Off,
    Branch,
    Author,
}

impl FilterMode {
    pub fn is_active(self) -> bool {
        self != FilterMode::Off
    }
}

pub fn map_key(key: KeyEvent, filter_mode: FilterMode) -> Action {
    if filter_mode.is_active() {
        return match key.code {
            KeyCode::Esc => Action::FilterCancel,
            KeyCode::Enter => Action::FilterConfirm,
            KeyCode::Backspace => Action::FilterBackspace,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,
            KeyCode::Char(c) => Action::FilterChar(c),
            _ => Action::None,
        };
    }

    match key.code {
        KeyCode::Char('q') => Action::Quit,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,
        KeyCode::Char('j') | KeyCode::Down => Action::ScrollDown,
        KeyCode::Char('k') | KeyCode::Up => Action::ScrollUp,
        KeyCode::Char('h') | KeyCode::Left => Action::PanelLeft,
        KeyCode::Char('l') | KeyCode::Right => Action::PanelRight,
        KeyCode::Char('H') => Action::ScrollLeft,
        KeyCode::Char('L') => Action::ScrollRight,
        KeyCode::Tab => Action::NextPane,
        KeyCode::BackTab => Action::PrevPane,
        KeyCode::Enter => Action::Select,
        KeyCode::Char('f') => Action::ToggleForks,
        KeyCode::Char('/') => Action::Filter,
        KeyCode::Char('a') => Action::AuthorFilter,
        KeyCode::Char('r') => Action::Refresh,
        KeyCode::Char('?') => Action::Help,
        KeyCode::Esc => Action::ClosePopup,
        _ => Action::None,
    }
}
