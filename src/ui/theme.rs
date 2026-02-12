use ratatui::style::Color;

pub const BRANCH_COLORS: &[Color] = &[
    Color::Green,
    Color::Cyan,
    Color::Magenta,
    Color::Yellow,
    Color::Blue,
    Color::Red,
    Color::LightGreen,
    Color::LightCyan,
    Color::LightMagenta,
    Color::LightYellow,
];

pub const HEAD_COLOR: Color = Color::Green;
pub const TAG_COLOR: Color = Color::Yellow;
pub const FORK_DIM: Color = Color::DarkGray;
pub const SELECTED_BG: Color = Color::Rgb(40, 40, 60);
pub const BORDER_COLOR: Color = Color::Rgb(80, 80, 100);
pub const STATUS_BG: Color = Color::Rgb(30, 30, 40);
pub const FILTER_COLOR: Color = Color::Cyan;

pub fn branch_color(index: usize) -> Color {
    BRANCH_COLORS[index % BRANCH_COLORS.len()]
}
