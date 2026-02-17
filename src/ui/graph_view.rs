use crate::git::types::CommitSource;
use crate::graph::layout::format_time_short;
use crate::graph::types::GraphRow;
use crate::ui::theme;
use ratatui::{
    buffer::Buffer as Buf,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};
use unicode_width::UnicodeWidthStr;

pub struct GraphView<'a> {
    pub rows: &'a [GraphRow],
    pub scroll_y: usize,
    pub scroll_x: usize,
    pub selected: usize,
    pub highlighted_oids: &'a std::collections::HashSet<crate::git::types::Oid>,
    pub is_active: bool,
    pub is_first_pane: bool,
    pub trunk_count: usize,
}

impl<'a> Widget for GraphView<'a> {
    fn render(self, area: Rect, buf: &mut Buf) {
        if area.height == 0 {
            return;
        }

        let area = if !self.is_first_pane {
            // Vertical separator on left edge for non-first panes
            let sep_style = Style::default().fg(theme::SEPARATOR);
            for y in area.y..area.bottom() {
                buf[(area.x, y)].set_char('\u{2503}');
                buf[(area.x, y)].set_style(sep_style);
            }
            Rect {
                x: area.x + 1,
                y: area.y,
                width: area.width.saturating_sub(1),
                height: area.height,
            }
        } else {
            area
        };

        let visible = area.height as usize;
        let avail_w = area.width as usize;
        let sel_bg = if self.is_active { theme::SELECTED_BG } else { theme::UNFOCUSED_SEL_BG };

        for (i, row) in self
            .rows
            .iter()
            .skip(self.scroll_y)
            .take(visible)
            .enumerate()
        {
            let y = area.y + i as u16;
            if y >= area.y + area.height {
                break;
            }

            let abs_idx = self.scroll_y + i;
            let is_selected = abs_idx == self.selected;
            let is_highlighted = self.highlighted_oids.contains(&row.oid);

            let line = build_row_line(row, is_selected, is_highlighted, self.scroll_x, avail_w, self.trunk_count, self.is_active);
            let line_width: usize = line
                .spans
                .iter()
                .map(|s| UnicodeWidthStr::width(s.content.as_ref()))
                .sum();

            buf.set_line(area.x, y, &line, area.width);

            if is_selected && line_width < avail_w {
                let fill_style = Style::default().bg(sel_bg);
                for x in (area.x + line_width as u16)..area.right() {
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
    is_active: bool,
) -> Line<'static> {
    let mut graph_spans: Vec<Span<'static>> = Vec::new();
    let mut text_spans: Vec<Span<'static>> = Vec::new();
    let is_fork = matches!(row.source, CommitSource::Fork(_));
    let sel_bg = if is_active { theme::SELECTED_BG } else { theme::UNFOCUSED_SEL_BG };

    // Selection indicator (▎ at column 0)
    if selected {
        let indicator_fg = if is_active { theme::ACTIVE_BORDER } else { theme::DIM_TEXT };
        graph_spans.push(Span::styled(
            "\u{258e}",
            Style::default().fg(indicator_fg).bg(sel_bg),
        ));
    } else {
        graph_spans.push(Span::raw(" "));
    }

    // Graph cells
    for cell in &row.cells {
        let color = if is_fork {
            theme::FORK_DIM
        } else {
            theme::branch_color_by_identity(cell.color_index, trunk_count)
        };
        let mut style = Style::default().fg(color);
        if selected {
            style = style.bg(sel_bg);
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

    // Reserve fixed time column (right-aligned, max 4 chars + 1 space padding)
    let time_str = format_time_short(&row.time);
    let time_col_w = 5; // e.g. " 12mo" or "   2h"
    let mut budget = avail_width.saturating_sub(graph_width).saturating_sub(time_col_w);

    // Branch labels (max 2, with [*name] for HEAD)
    let commit_color_idx = row.cells.first().map(|c| c.color_index).unwrap_or(0);
    let mut shown_branches = 0;
    let max_branches = 2;
    for (i, name) in row.branch_names.iter().enumerate() {
        if shown_branches >= max_branches {
            break;
        }
        if budget < 4 {
            break;
        }
        let max_label = budget.min(20);
        let is_head_branch = row.is_head && i == 0;
        let prefix = if is_head_branch { "*" } else { "" };
        let display = format!("{prefix}{name}");
        let label = truncate_with_ellipsis(&display, max_label.saturating_sub(3));
        let formatted = format!("[{label}] ");
        let w = UnicodeWidthStr::width(formatted.as_str());
        let style = Style::default()
            .fg(theme::branch_color_by_identity(commit_color_idx, trunk_count))
            .add_modifier(Modifier::BOLD);
        text_spans.push(Span::styled(formatted, style));
        budget = budget.saturating_sub(w);
        shown_branches += 1;
    }

    // Overflow indicator
    let overflow = row.branch_names.len().saturating_sub(max_branches);
    if overflow > 0 && budget >= 5 {
        let overflow_str = format!("[+{overflow}] ");
        let w = UnicodeWidthStr::width(overflow_str.as_str());
        text_spans.push(Span::styled(
            overflow_str,
            Style::default().fg(theme::DIM_TEXT),
        ));
        budget = budget.saturating_sub(w);
    }

    // Commit message
    let author_str = format!(" {}", row.author);
    let author_w = UnicodeWidthStr::width(author_str.as_str());
    let msg_budget = if budget > author_w + 5 {
        budget - author_w
    } else {
        budget
    };

    if msg_budget > 0 {
        let msg = truncate_with_ellipsis(&row.message, msg_budget);
        let msg_w = UnicodeWidthStr::width(msg.as_str());
        let msg_style = if selected {
            Style::default().bg(sel_bg)
        } else if is_fork {
            Style::default().fg(theme::FORK_DIM)
        } else {
            Style::default()
        };
        text_spans.push(Span::styled(msg, msg_style));
        budget = budget.saturating_sub(msg_w);
    }

    // Author (dim)
    if budget >= author_w && !row.author.is_empty() {
        let style = if selected {
            Style::default().fg(theme::DIM_TEXT).bg(sel_bg)
        } else {
            Style::default().fg(theme::DIM_TEXT)
        };
        text_spans.push(Span::styled(author_str, style));
        budget = budget.saturating_sub(author_w);
    }

    // Tags without emoji — (name) in TAG_COLOR
    for name in &row.tag_names {
        let formatted = format!("({name}) ");
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

    // Apply scroll_x only to text portion
    if scroll_x > 0 {
        text_spans = skip_chars_preserving_style(text_spans, scroll_x);
    }

    let mut spans = graph_spans;
    spans.extend(text_spans);

    // Right-aligned time column
    let current_width: usize = spans
        .iter()
        .map(|s| UnicodeWidthStr::width(s.content.as_ref()))
        .sum();
    if avail_width > current_width + time_str.len() {
        let padding = avail_width - current_width - time_str.len();
        if padding > 0 {
            let pad_style = if selected {
                Style::default().bg(sel_bg)
            } else {
                Style::default()
            };
            spans.push(Span::styled(" ".repeat(padding), pad_style));
        }
        let time_style = if selected {
            Style::default().fg(theme::DIM_TEXT).bg(sel_bg)
        } else {
            Style::default().fg(theme::DIM_TEXT)
        };
        spans.push(Span::styled(time_str, time_style));
    }

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
