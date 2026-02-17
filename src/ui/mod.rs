pub mod branch_panel;
pub mod detail_panel;
pub mod graph_view;
pub mod header_bar;
pub mod help_panel;
pub mod input;
pub mod status_bar;
pub mod theme;
pub mod toast;

use ratatui::layout::{Constraint, Layout, Rect};

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
