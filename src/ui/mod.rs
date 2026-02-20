pub mod branch_panel;
pub mod config_screen;
pub mod detail_panel;
pub mod graph_view;
pub mod header_bar;
pub mod help_panel;
pub mod input;
pub mod status_bar;
pub mod theme;
pub mod toast;

use ratatui::layout::{Constraint, Layout, Rect};
use unicode_width::UnicodeWidthStr;

pub fn truncate_with_ellipsis(s: &str, max: usize) -> String {
    if UnicodeWidthStr::width(s) <= max {
        return s.to_string();
    }
    if max <= 1 {
        return "\u{2026}".to_string();
    }
    let mut result = String::new();
    let mut w = 0;
    for ch in s.chars() {
        let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if w + cw > max - 1 {
            break;
        }
        result.push(ch);
        w += cw;
    }
    result.push('\u{2026}');
    result
}

pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vert = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Min(0),
    ])
    .split(area);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Min(0),
    ])
    .split(vert[1])[1]
}
