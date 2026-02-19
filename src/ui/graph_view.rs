use crate::git::types::CommitSource;
use crate::graph::layout::format_time_short;
use crate::graph::types::{Cell, CellSymbol, GraphRow};
use crate::ui::theme::ThemePalette;
use ratatui::{
    buffer::Buffer as Buf,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};
use std::collections::HashMap;
use unicode_width::UnicodeWidthStr;

const TRUNK_VERT_CHARS: &[&str] = &["┃ ", "╏ ", "┇ "];

fn cell_glyph(cell: &Cell) -> &'static str {
    match cell.symbol {
        CellSymbol::Commit => "◯ ",
        CellSymbol::Vertical => {
            if let Some(ti) = cell.trunk_index {
                TRUNK_VERT_CHARS.get(ti).copied().unwrap_or("┃ ")
            } else {
                "│ "
            }
        }
        CellSymbol::HorizontalLeft => "──",
        CellSymbol::HorizontalRight => "──",
        CellSymbol::MergeDown => "╭─",
        CellSymbol::MergeUp => "╰─",
        CellSymbol::BranchRight => "─╮",
        CellSymbol::BranchLeft => "─╯",
        CellSymbol::Empty => "  ",
    }
}

pub struct GraphView<'a> {
    pub rows: &'a [GraphRow],
    pub scroll_y: usize,
    pub scroll_x: usize,
    pub selected: usize,
    pub highlighted_oids: &'a std::collections::HashSet<crate::git::types::Oid>,
    pub is_active: bool,
    pub trunk_count: usize,
    pub palette: &'a ThemePalette,
    pub branch_index_to_name: &'a HashMap<usize, String>,
}

impl<'a> Widget for GraphView<'a> {
    fn render(self, area: Rect, buf: &mut Buf) {
        if area.height == 0 {
            return;
        }

        let bg_style = Style::default().bg(self.palette.content_bg);
        for y in area.y..area.bottom() {
            for x in area.x..area.right() {
                buf[(x, y)].set_style(bg_style);
            }
        }

        let avail_w = area.width as usize;

        let header_y = area.y;
        render_lane_header(
            buf,
            header_y,
            area.x,
            avail_w,
            self.rows.get(self.scroll_y),
            self.branch_index_to_name,
            self.trunk_count,
            self.palette,
        );

        let commit_area_top = area.y + 1;
        let visible = (area.height as usize).saturating_sub(1);
        let sel_bg = if self.is_active {
            self.palette.selected_bg
        } else {
            self.palette.unfocused_sel_bg
        };

        for (i, row) in self
            .rows
            .iter()
            .skip(self.scroll_y)
            .take(visible)
            .enumerate()
        {
            let y = commit_area_top + i as u16;
            if y >= area.y + area.height {
                break;
            }

            let abs_idx = self.scroll_y + i;
            let is_selected = abs_idx == self.selected;
            let is_highlighted = self.highlighted_oids.contains(&row.oid);

            let line = build_row_line(
                row,
                is_selected,
                is_highlighted,
                self.scroll_x,
                avail_w,
                self.trunk_count,
                self.is_active,
                self.branch_index_to_name,
                self.palette,
            );
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

#[allow(clippy::too_many_arguments)]
fn render_lane_header(
    buf: &mut Buf,
    y: u16,
    x_start: u16,
    avail_w: usize,
    first_visible_row: Option<&GraphRow>,
    branch_index_to_name: &HashMap<usize, String>,
    trunk_count: usize,
    palette: &ThemePalette,
) {
    let header_bg = Style::default().bg(palette.lane_header_bg);
    for x in x_start..(x_start + avail_w as u16) {
        buf[(x, y)].set_style(header_bg);
    }

    let lane_branches = match first_visible_row {
        Some(row) => &row.lane_branches,
        None => return,
    };

    let indicator_offset = 1u16;

    let mut labels: Vec<(u16, &str, ratatui::style::Color)> = Vec::new();
    for (col_idx, slot) in lane_branches.iter().enumerate() {
        if let Some(bi) = slot {
            if let Some(name) = branch_index_to_name.get(bi) {
                let x_pos = indicator_offset
                    + (col_idx.min(u16::MAX as usize / 2) as u16).saturating_mul(2);
                let color = palette.branch_color_by_identity(*bi, trunk_count);
                labels.push((x_pos, name, color));
            }
        }
    }

    labels.sort_by_key(|&(x, _, _)| x);

    let mut next_free_x = 0u16;
    for (x_pos, name, color) in labels {
        let abs_x = x_start.saturating_add(x_pos);
        if abs_x < next_free_x {
            continue;
        }
        let max_chars = (x_start + avail_w as u16).saturating_sub(abs_x) as usize;
        if max_chars == 0 {
            break;
        }
        let display = truncate_with_ellipsis(name, max_chars);
        let style = Style::default()
            .fg(color)
            .bg(palette.lane_header_bg)
            .add_modifier(Modifier::DIM);
        for (ci, ch) in display.chars().enumerate() {
            let cx = abs_x + ci as u16;
            if cx >= x_start + avail_w as u16 {
                break;
            }
            buf[(cx, y)].set_char(ch);
            buf[(cx, y)].set_style(style);
        }
        next_free_x = abs_x + UnicodeWidthStr::width(display.as_str()) as u16 + 1;
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

#[allow(clippy::too_many_arguments)]
fn build_row_line(
    row: &GraphRow,
    selected: bool,
    highlighted: bool,
    scroll_x: usize,
    avail_width: usize,
    trunk_count: usize,
    is_active: bool,
    branch_index_to_name: &HashMap<usize, String>,
    palette: &ThemePalette,
) -> Line<'static> {
    let mut graph_spans: Vec<Span<'static>> = Vec::new();
    let mut text_spans: Vec<Span<'static>> = Vec::new();
    let is_fork = matches!(row.source, CommitSource::Fork(_));
    let sel_bg = if is_active {
        palette.selected_bg
    } else {
        palette.unfocused_sel_bg
    };

    if selected {
        let indicator_fg = if is_active {
            palette.selected_accent
        } else {
            palette.dim_text
        };
        graph_spans.push(Span::styled(
            "\u{258e}",
            Style::default().fg(indicator_fg).bg(sel_bg),
        ));
    } else {
        graph_spans.push(Span::raw(" "));
    }

    for cell in &row.cells {
        let color = if is_fork {
            palette.fork_dim
        } else {
            palette.branch_color_by_identity(cell.color_index, trunk_count)
        };
        let mut style = Style::default().fg(color);
        if selected {
            style = style.bg(sel_bg);
        }
        if highlighted {
            style = style.add_modifier(Modifier::BOLD);
        }
        graph_spans.push(Span::styled(cell_glyph(cell), style));
    }
    graph_spans.push(Span::raw(" "));

    let graph_width: usize = graph_spans
        .iter()
        .map(|s| UnicodeWidthStr::width(s.content.as_ref()))
        .sum();

    let time_str = format_time_short(&row.time);
    let time_col_w = 5;
    let mut budget = avail_width
        .saturating_sub(graph_width)
        .saturating_sub(time_col_w);

    let commit_color_idx = row.cells.first().map(|c| c.color_index).unwrap_or(0);
    let max_branches = 2;
    let mut showed_branch_label = false;
    for (i, name) in row.branch_names.iter().enumerate() {
        if i >= max_branches || budget < 4 {
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
            .fg(palette.branch_color_by_identity(commit_color_idx, trunk_count))
            .add_modifier(Modifier::BOLD);
        text_spans.push(Span::styled(formatted, style));
        budget = budget.saturating_sub(w);
        showed_branch_label = true;
    }

    let overflow = row.branch_names.len().saturating_sub(max_branches);
    if overflow > 0 && budget >= 5 {
        let overflow_str = format!("[+{overflow}] ");
        let w = UnicodeWidthStr::width(overflow_str.as_str());
        text_spans.push(Span::styled(
            overflow_str,
            Style::default().fg(palette.dim_text),
        ));
        budget = budget.saturating_sub(w);
        showed_branch_label = true;
    }

    if !showed_branch_label && (row.is_merge || row.is_fork_point) {
        if let Some(bi) = row.branch_index {
            if let Some(name) = branch_index_to_name.get(&bi) {
                let max_label = budget.min(18);
                if max_label >= 4 {
                    let label = truncate_with_ellipsis(name, max_label.saturating_sub(3));
                    let formatted = format!("[{label}] ");
                    let w = UnicodeWidthStr::width(formatted.as_str());
                    let color = palette.branch_color_by_identity(bi, trunk_count);
                    let style = Style::default().fg(color).add_modifier(Modifier::DIM);
                    text_spans.push(Span::styled(formatted, style));
                    budget = budget.saturating_sub(w);
                }
            }
        }
    }

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
            Style::default().fg(palette.fork_dim)
        } else {
            Style::default()
        };
        text_spans.push(Span::styled(msg, msg_style));
        budget = budget.saturating_sub(msg_w);
    }

    if budget >= author_w && !row.author.is_empty() {
        let style = if selected {
            Style::default().fg(palette.dim_text).bg(sel_bg)
        } else {
            Style::default().fg(palette.dim_text)
        };
        text_spans.push(Span::styled(author_str, style));
        budget = budget.saturating_sub(author_w);
    }

    for name in &row.tag_names {
        let formatted = format!("({name}) ");
        let w = UnicodeWidthStr::width(formatted.as_str());
        if budget < w {
            break;
        }
        let style = Style::default()
            .fg(palette.tag_color)
            .add_modifier(Modifier::BOLD);
        text_spans.push(Span::styled(formatted, style));
        budget = budget.saturating_sub(w);
    }
    let _ = budget;

    if scroll_x > 0 {
        text_spans = skip_chars_preserving_style(text_spans, scroll_x);
    }

    let mut spans = graph_spans;
    spans.extend(text_spans);

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
            Style::default().fg(palette.dim_text).bg(sel_bg)
        } else {
            Style::default().fg(palette.dim_text)
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
