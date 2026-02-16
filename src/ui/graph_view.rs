use crate::git::types::CommitSource;
use crate::graph::layout::format_time_ago;
use crate::graph::types::GraphRow;
use crate::ui::theme;
use ratatui::{
    buffer::Buffer as Buf,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Widget},
};
use unicode_width::UnicodeWidthStr;

pub struct GraphView<'a> {
    pub rows: &'a [GraphRow],
    pub scroll_y: usize,
    pub scroll_x: usize,
    pub selected: usize,
    pub highlighted_oids: &'a std::collections::HashSet<crate::git::types::Oid>,
    pub repo_name: &'a str,
    pub is_active: bool,
    pub is_first_pane: bool,
    pub trunk_count: usize,
}

impl<'a> Widget for GraphView<'a> {
    fn render(self, area: Rect, buf: &mut Buf) {
        if area.height < 2 {
            return;
        }

        let area = if !self.is_first_pane {
            let border = Block::default()
                .borders(Borders::LEFT)
                .border_style(Style::default().fg(theme::BORDER_COLOR));
            let inner = border.inner(area);
            border.render(area, buf);
            inner
        } else {
            area
        };

        let header_style = if self.is_active {
            Style::default()
                .fg(theme::FILTER_COLOR)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::BORDER_COLOR)
        };

        let header = format!(" {} ", self.repo_name);
        let header_line = Line::from(Span::styled(header, header_style));

        let header_bg = if self.is_active {
            Style::default().bg(theme::STATUS_BG)
        } else {
            Style::default()
        };
        for x in area.x..area.right() {
            buf[(x, area.y)].set_style(header_bg);
        }
        buf.set_line(area.x, area.y, &header_line, area.width);

        let graph_area = Rect {
            x: area.x,
            y: area.y + 1,
            width: area.width,
            height: area.height - 1,
        };
        let visible = graph_area.height as usize;
        let avail_w = graph_area.width as usize;

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

            let line = build_row_line(row, is_selected, is_highlighted, self.scroll_x, avail_w, self.trunk_count);
            let line_width: usize = line
                .spans
                .iter()
                .map(|s| UnicodeWidthStr::width(s.content.as_ref()))
                .sum();

            buf.set_line(graph_area.x, y, &line, graph_area.width);

            if is_selected && line_width < avail_w {
                let fill_style = Style::default().bg(theme::SELECTED_BG);
                for x in (graph_area.x + line_width as u16)..graph_area.right() {
                    buf[(x, y)].set_style(fill_style);
                }
            }
        }
    }
}

fn truncate_with_ellipsis(s: &str, max: usize) -> String {
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

fn build_row_line(
    row: &GraphRow,
    selected: bool,
    highlighted: bool,
    scroll_x: usize,
    avail_width: usize,
    trunk_count: usize,
) -> Line<'static> {
    let mut graph_spans: Vec<Span<'static>> = Vec::new();
    let mut text_spans: Vec<Span<'static>> = Vec::new();
    let is_fork = matches!(row.source, CommitSource::Fork(_));

    // 1. Graph cells (fixed, never scrolled)
    for cell in &row.cells {
        let color = if is_fork {
            theme::FORK_DIM
        } else {
            theme::branch_color_by_identity(cell.color_index, trunk_count)
        };
        let mut style = Style::default().fg(color);
        if selected {
            style = style.bg(theme::SELECTED_BG);
        }
        if highlighted {
            style = style.add_modifier(Modifier::BOLD);
        }
        graph_spans.push(Span::styled(cell.to_chars(), style));
    }
    graph_spans.push(Span::raw(" "));

    let graph_width: usize = graph_spans
        .iter()
        .map(|s| UnicodeWidthStr::width(s.content.as_ref()))
        .sum();
    let mut budget = avail_width.saturating_sub(graph_width);

    // 2. HEAD marker
    if row.is_head && budget >= 5 {
        let style = Style::default()
            .fg(theme::HEAD_COLOR)
            .add_modifier(Modifier::BOLD);
        text_spans.push(Span::styled("HEAD ", style));
        budget = budget.saturating_sub(5);
    }

    // 3. First branch label (cap 20 chars)
    let commit_color_idx = row.cells.first().map(|c| c.color_index).unwrap_or(0);
    if let Some(name) = row.branch_names.first() {
        if budget >= 4 {
            let max_label = budget.min(20);
            let label = truncate_with_ellipsis(name, max_label.saturating_sub(3));
            let formatted = format!("[{label}] ");
            let w = UnicodeWidthStr::width(formatted.as_str());
            let style = Style::default()
                .fg(theme::branch_color_by_identity(commit_color_idx, trunk_count))
                .add_modifier(Modifier::BOLD);
            text_spans.push(Span::styled(formatted, style));
            budget = budget.saturating_sub(w);
        }
    }

    // 4. Commit message (fill remaining, leave room for time)
    let time_ago = format_time_ago(&row.time);
    let time_str = format!(" ({time_ago})");
    let time_w = UnicodeWidthStr::width(time_str.as_str());

    let msg_budget = if budget > time_w + 5 {
        budget - time_w
    } else {
        budget
    };

    if msg_budget > 0 {
        let msg = truncate_with_ellipsis(&row.message, msg_budget);
        let msg_w = UnicodeWidthStr::width(msg.as_str());
        let msg_style = if selected {
            Style::default().bg(theme::SELECTED_BG)
        } else if is_fork {
            Style::default().fg(theme::FORK_DIM)
        } else {
            Style::default()
        };
        text_spans.push(Span::styled(msg, msg_style));
        budget = budget.saturating_sub(msg_w);
    }

    // 5. Time ago
    if budget >= time_w {
        let time_style = if selected {
            Style::default()
                .fg(ratatui::style::Color::DarkGray)
                .bg(theme::SELECTED_BG)
        } else {
            Style::default().fg(ratatui::style::Color::DarkGray)
        };
        text_spans.push(Span::styled(time_str, time_style));
        budget = budget.saturating_sub(time_w);
    }

    // 6. Extra branch/tag labels
    for name in row.branch_names.iter().skip(1) {
        let formatted = format!("[{name}] ");
        let w = UnicodeWidthStr::width(formatted.as_str());
        if budget < w {
            break;
        }
        let style = Style::default()
            .fg(theme::branch_color_by_identity(commit_color_idx, trunk_count))
            .add_modifier(Modifier::BOLD);
        text_spans.push(Span::styled(formatted, style));
        budget = budget.saturating_sub(w);
    }

    for name in &row.tag_names {
        let formatted = format!("\u{1f3f7} {name} ");
        let w = UnicodeWidthStr::width(formatted.as_str());
        if budget < w {
            break;
        }
        let style = Style::default()
            .fg(theme::TAG_COLOR)
            .add_modifier(Modifier::BOLD);
        text_spans.push(Span::styled(formatted, style));
        budget = budget.saturating_sub(w);
    }
    let _ = budget;

    // Apply scroll_x only to text portion (graph cells stay fixed)
    if scroll_x > 0 {
        text_spans = skip_chars_preserving_style(text_spans, scroll_x);
    }

    let mut spans = graph_spans;
    spans.extend(text_spans);
    Line::from(spans)
}

fn skip_chars_preserving_style(spans: Vec<Span<'static>>, skip: usize) -> Vec<Span<'static>> {
    let mut remaining = skip;
    let mut result = Vec::new();

    for span in spans {
        if remaining == 0 {
            result.push(span);
            continue;
        }

        let span_w = UnicodeWidthStr::width(span.content.as_ref());
        if span_w <= remaining {
            remaining -= span_w;
            continue;
        }

        let mut chars_to_skip = 0;
        let mut skipped_w = 0;
        for ch in span.content.chars() {
            let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
            if skipped_w + cw > remaining {
                break;
            }
            skipped_w += cw;
            chars_to_skip += 1;
        }
        remaining = 0;

        let rest: String = span.content.chars().skip(chars_to_skip).collect();
        if !rest.is_empty() {
            result.push(Span::styled(rest, span.style));
        }
    }

    result
}
