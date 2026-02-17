use crate::data_source::ViewMode;
use ratatui::style::Color;

pub const BRANCH_COLORS: &[Color] = &[
    Color::Green,
    Color::Cyan,
    Color::Magenta,
    Color::Yellow,
    Color::Blue,
    Color::Red,
];

pub const HEAD_COLOR: Color = Color::Green;
pub const TAG_COLOR: Color = Color::Yellow;
pub const FORK_DIM: Color = Color::DarkGray;
pub const SELECTED_BG: Color = Color::Rgb(50, 50, 80);
pub const UNFOCUSED_SEL_BG: Color = Color::Rgb(38, 38, 55);
pub const STATUS_BG: Color = Color::Rgb(30, 30, 40);
pub const ACCENT: Color = Color::Rgb(140, 115, 200);
pub const HEADER_BG: Color = Color::Rgb(25, 25, 38);
pub const SEPARATOR: Color = Color::Rgb(55, 55, 75);
pub const PANEL_LABEL: Color = Color::Rgb(100, 95, 130);
pub const FILTER_COLOR: Color = Color::Cyan;
pub const DIM_TEXT: Color = Color::Rgb(100, 100, 120);
pub const ACTIVE_BORDER: Color = Color::Rgb(120, 120, 180);

pub const TRUNK_COLORS: &[Color] = &[Color::LightCyan, Color::LightYellow, Color::LightGreen];

pub const SECTION_SEPARATOR: Color = Color::Rgb(60, 60, 80);
pub const DIM_PREFIX: Color = Color::Rgb(90, 90, 110);
pub const ERROR_FG: Color = Color::LightRed;
pub const SECTION_HEADER_FG: Color = Color::White;

#[derive(Clone)]
#[allow(dead_code)]
pub struct ThemePalette {
    pub chrome_bg: Color,
    pub chrome_fg: Color,
    pub content_bg: Color,
    pub content_fg: Color,
    pub selected_bg: Color,
    pub header_bg: Color,
    pub status_bg: Color,
}

pub fn palette_for_mode(mode: &ViewMode) -> ThemePalette {
    match mode {
        ViewMode::Local => ThemePalette {
            chrome_bg: Color::Rgb(25, 25, 38),
            chrome_fg: Color::Rgb(100, 95, 130),
            content_bg: Color::Rgb(35, 35, 52),
            content_fg: Color::Rgb(220, 220, 230),
            selected_bg: Color::Rgb(50, 50, 80),
            header_bg: Color::Rgb(25, 25, 38),
            status_bg: Color::Rgb(30, 30, 40),
        },
        ViewMode::Remote => ThemePalette {
            chrome_bg: Color::Rgb(25, 25, 38),
            chrome_fg: Color::Rgb(100, 95, 130),
            content_bg: Color::Rgb(18, 18, 28),
            content_fg: Color::Rgb(170, 170, 190),
            selected_bg: Color::Rgb(40, 40, 65),
            header_bg: Color::Rgb(25, 25, 38),
            status_bg: Color::Rgb(22, 22, 32),
        },
    }
}

pub fn branch_prefix_color(name: &str) -> Color {
    if name.starts_with("feat/") || name.starts_with("feature/") {
        Color::Cyan
    } else if name.starts_with("fix/") || name.starts_with("bugfix/") || name.starts_with("hotfix/")
    {
        Color::LightRed
    } else if name.starts_with("chore/") {
        Color::Rgb(140, 140, 160)
    } else if name.starts_with("release/") {
        Color::Yellow
    } else if name.starts_with("cherry-pick/") {
        Color::Magenta
    } else {
        Color::White
    }
}

pub fn branch_color_by_identity(branch_index: usize, trunk_count: usize) -> Color {
    if branch_index < trunk_count && branch_index < TRUNK_COLORS.len() {
        TRUNK_COLORS[branch_index]
    } else {
        BRANCH_COLORS[branch_index.saturating_sub(trunk_count) % BRANCH_COLORS.len()]
    }
}
