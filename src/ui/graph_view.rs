use crate::git::types::CommitSource;
use crate::graph::types::GraphRow;
use crate::ui::theme;
use ratatui::{
    buffer::Buffer as Buf,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};

pub struct GraphView<'a> {
    pub rows: &'a [GraphRow],
    pub scroll_y: usize,
    pub scroll_x: usize,
    pub selected: usize,
    pub highlighted_oids: &'a std::collections::HashSet<crate::git::types::Oid>,
    pub repo_name: &'a str,
    pub is_active: bool,
}

impl<'a> Widget for GraphView<'a> {
    fn render(self, area: Rect, buf: &mut Buf) {
        if area.height < 2 {
            return;
        }

        // Render pane header (1 row)
        let header_style = if self.is_active {
            Style::default()
                .fg(theme::FILTER_COLOR)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::BORDER_COLOR)
        };

        let header = format!(" {} ", self.repo_name);
        let header_line = Line::from(Span::styled(header, header_style));

        // Fill header bg
        let header_bg = if self.is_active {
            Style::default().bg(theme::STATUS_BG)
        } else {
            Style::default()
        };
        for x in area.x..area.right() {
            buf[(x, area.y)].set_style(header_bg);
        }
        buf.set_line(area.x, area.y, &header_line, area.width);

        // Graph rows below header
        let graph_area = Rect {
            x: area.x,
            y: area.y + 1,
            width: area.width,
            height: area.height - 1,
        };
        let visible = graph_area.height as usize;

        for (i, row) in self
            .rows
            .iter()
            .skip(self.scroll_y)
            .take(visible)
            .enumerate()
        {
            let y = graph_area.y + i as u16;
            if y >= graph_area.y + graph_area.height {
                break;
            }

            let abs_idx = self.scroll_y + i;
            let is_selected = abs_idx == self.selected && self.is_active;
            let is_highlighted = self.highlighted_oids.contains(&row.oid);

            let line = build_row_line(row, is_selected, is_highlighted, self.scroll_x);
            let line_width: usize = line.spans.iter().map(|s| s.content.len()).sum();

            buf.set_line(graph_area.x, y, &line, graph_area.width);

            if is_selected && line_width < graph_area.width as usize {
                let fill_style = Style::default().bg(theme::SELECTED_BG);
                for x in (graph_area.x + line_width as u16)..graph_area.right() {
                    buf[(x, y)].set_style(fill_style);
                }
            }
        }
    }
}

fn build_row_line(
    row: &GraphRow,
    selected: bool,
    highlighted: bool,
    scroll_x: usize,
) -> Line<'static> {
    let mut spans = Vec::new();
    let is_fork = matches!(row.source, CommitSource::Fork(_));

    for cell in &row.cells {
        let color = if is_fork {
            theme::FORK_DIM
        } else {
            theme::branch_color(cell.color_index)
        };

        let mut style = Style::default().fg(color);
        if selected {
            style = style.bg(theme::SELECTED_BG);
        }
        if highlighted {
            style = style.add_modifier(Modifier::BOLD);
        }

        spans.push(Span::styled(cell.to_chars().to_string(), style));
    }

    spans.push(Span::raw(" "));

    if row.is_head {
        let style = Style::default()
            .fg(theme::HEAD_COLOR)
            .add_modifier(Modifier::BOLD);
        spans.push(Span::styled("HEAD ", style));
    }

    for name in &row.branch_names {
        let style = Style::default()
            .fg(theme::branch_color(0))
            .add_modifier(Modifier::BOLD);
        spans.push(Span::styled(format!("[{name}] "), style));
    }

    for name in &row.tag_names {
        let style = Style::default()
            .fg(theme::TAG_COLOR)
            .add_modifier(Modifier::BOLD);
        spans.push(Span::styled(format!("🏷 {name} "), style));
    }

    let msg_style = if selected {
        Style::default().bg(theme::SELECTED_BG)
    } else if is_fork {
        Style::default().fg(theme::FORK_DIM)
    } else {
        Style::default()
    };
    spans.push(Span::styled(row.message.clone(), msg_style));

    let time_style = Style::default().fg(ratatui::style::Color::DarkGray);
    if selected {
        spans.push(Span::styled(
            format!(" ({})", row.time_ago),
            time_style.bg(theme::SELECTED_BG),
        ));
    } else {
        spans.push(Span::styled(format!(" ({})", row.time_ago), time_style));
    }

    let mut line = Line::from(spans);

    if scroll_x > 0 {
        let full: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        if scroll_x < full.len() {
            let trimmed = &full[scroll_x..];
            line = Line::from(trimmed.to_string());
        }
    }

    line
}
