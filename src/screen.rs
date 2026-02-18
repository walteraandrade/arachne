use crate::config::Config;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigSection {
    Repos,
    Profiles,
    Theme,
    Trunk,
}

impl ConfigSection {
    pub const ALL: &[ConfigSection] = &[
        ConfigSection::Repos,
        ConfigSection::Profiles,
        ConfigSection::Theme,
        ConfigSection::Trunk,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            ConfigSection::Repos => "Repos",
            ConfigSection::Profiles => "Profiles",
            ConfigSection::Theme => "Theme",
            ConfigSection::Trunk => "Trunk",
        }
    }

    pub fn index(&self) -> usize {
        match self {
            ConfigSection::Repos => 0,
            ConfigSection::Profiles => 1,
            ConfigSection::Theme => 2,
            ConfigSection::Trunk => 3,
        }
    }

    pub fn from_index(i: usize) -> Self {
        match i % 4 {
            0 => ConfigSection::Repos,
            1 => ConfigSection::Profiles,
            2 => ConfigSection::Theme,
            _ => ConfigSection::Trunk,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldMode {
    Navigate,
    Editing(String),
}

pub struct ConfigScreenState {
    pub active_section: ConfigSection,
    pub cursor: usize,
    pub field_mode: FieldMode,
    pub draft: Config,
    pub dirty: bool,
    pub first_launch: bool,
}

impl ConfigScreenState {
    pub fn new(config: &Config) -> Self {
        Self {
            active_section: ConfigSection::Repos,
            cursor: 0,
            field_mode: FieldMode::Navigate,
            draft: config.clone(),
            dirty: false,
            first_launch: false,
        }
    }

    pub fn first_launch(config: &Config) -> Self {
        let mut state = Self::new(config);
        state.first_launch = true;
        state
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> ConfigAction {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return ConfigAction::Quit;
        }

        match &self.field_mode {
            FieldMode::Editing(ref _text) => self.handle_editing_key(key),
            FieldMode::Navigate => self.handle_navigate_key(key),
        }
    }

    fn handle_navigate_key(&mut self, key: KeyEvent) -> ConfigAction {
        match key.code {
            KeyCode::Esc => {
                if self.first_launch {
                    return ConfigAction::QuitConfirm;
                }
                ConfigAction::Close
            }
            KeyCode::Tab => {
                let idx = self.active_section.index();
                self.active_section = ConfigSection::from_index(idx + 1);
                self.cursor = 0;
                ConfigAction::None
            }
            KeyCode::BackTab => {
                let idx = self.active_section.index();
                self.active_section = ConfigSection::from_index(if idx == 0 { 3 } else { idx - 1 });
                self.cursor = 0;
                ConfigAction::None
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.cursor = self.cursor.saturating_add(1);
                ConfigAction::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.cursor = self.cursor.saturating_sub(1);
                ConfigAction::None
            }
            KeyCode::Enter => {
                self.start_edit();
                ConfigAction::None
            }
            KeyCode::Char('a') => ConfigAction::AddItem,
            KeyCode::Char('x') => ConfigAction::RemoveItem,
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                ConfigAction::Save
            }
            _ => ConfigAction::None,
        }
    }

    fn handle_editing_key(&mut self, key: KeyEvent) -> ConfigAction {
        match key.code {
            KeyCode::Esc => {
                self.field_mode = FieldMode::Navigate;
                ConfigAction::None
            }
            KeyCode::Enter => {
                self.confirm_edit();
                ConfigAction::None
            }
            KeyCode::Backspace => {
                if let FieldMode::Editing(ref mut text) = self.field_mode {
                    text.pop();
                    self.dirty = true;
                }
                ConfigAction::None
            }
            KeyCode::Char(c) => {
                if let FieldMode::Editing(ref mut text) = self.field_mode {
                    text.push(c);
                    self.dirty = true;
                }
                ConfigAction::None
            }
            _ => ConfigAction::None,
        }
    }

    fn start_edit(&mut self) {
        let current_value = self.current_field_value();
        if let Some(val) = current_value {
            self.field_mode = FieldMode::Editing(val);
        }
    }

    pub fn start_edit_public(&mut self) {
        self.start_edit();
    }

    fn confirm_edit(&mut self) {
        if let FieldMode::Editing(ref text) = self.field_mode {
            let text = text.clone();
            self.set_current_field_value(&text);
            self.dirty = true;
        }
        self.field_mode = FieldMode::Navigate;
    }

    fn current_field_value(&self) -> Option<String> {
        match self.active_section {
            ConfigSection::Repos => {
                let repos = self.draft.resolved_repos();
                repos.get(self.cursor).map(|r| r.path.to_string_lossy().to_string())
            }
            ConfigSection::Trunk => {
                self.draft.trunk_branches.get(self.cursor).cloned()
            }
            ConfigSection::Theme | ConfigSection::Profiles => None,
        }
    }

    fn set_current_field_value(&mut self, value: &str) {
        match self.active_section {
            ConfigSection::Repos => {
                if let Some(entry) = self.draft.repos.get_mut(self.cursor) {
                    entry.path = std::path::PathBuf::from(value);
                }
            }
            ConfigSection::Trunk => {
                if let Some(entry) = self.draft.trunk_branches.get_mut(self.cursor) {
                    *entry = value.to_string();
                }
            }
            ConfigSection::Theme | ConfigSection::Profiles => {}
        }
    }

    pub fn item_count(&self) -> usize {
        match self.active_section {
            ConfigSection::Repos => self.draft.resolved_repos().len(),
            ConfigSection::Trunk => self.draft.trunk_branches.len(),
            ConfigSection::Theme => 5,
            ConfigSection::Profiles => self.draft.profiles.len().max(1),
        }
    }

    pub fn clamp_cursor(&mut self) {
        let count = self.item_count();
        if count == 0 {
            self.cursor = 0;
        } else {
            self.cursor = self.cursor.min(count - 1);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigAction {
    None,
    Close,
    Save,
    Quit,
    QuitConfirm,
    AddItem,
    RemoveItem,
}

pub enum Screen {
    Graph,
    Config(ConfigScreenState),
}
