use crate::git::types::CommitSource;
use crate::graph::image_cache::ImageCache;
use crate::graph::layout::format_time_short;
use crate::graph::pixel_renderer::{RenderParams, COLS_PER_LANE};
use crate::graph::types::{Cell, CellSymbol, GraphRow};
use crate::kitty_protocol::encode_kitty_image;
use crate::terminal_graphics::GraphicsCapability;
use crate::ui::theme::ThemePalette;
use ratatui::{
    buffer::Buffer as Buf,
    layout::{Position, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
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
    pub max_lanes: usize,
    pub palette: &'a ThemePalette,
    pub branch_index_to_name: &'a HashMap<usize, String>,
    pub graphics_cap: &'a GraphicsCapability,
    pub image_cache: &'a mut ImageCache,
    pub render_params: Option<&'a RenderParams>,
}

impl<'a> GraphView<'a> {
    pub fn render_into(mut self, area: Rect, buf: &mut Buf) {
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

        let use_kitty = self.graphics_cap.is_kitty() && self.render_params.is_some();

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
            let is_highlighted = self.highlighted_oids.contains(&row.meta.oid);

            if let (true, Some(params)) = (use_kitty, self.render_params) {
                self.render_kitty_row(
                    buf,
                    area.x,
                    y,
                    avail_w,
                    row,
                    is_selected,
                    is_highlighted,
                    params,
                    sel_bg,
                );
            } else {
                let line = build_row_line(
                    row,
                    is_selected,
                    is_highlighted,
                    self.scroll_x,
                    avail_w,
                    self.trunk_count,
                    self.max_lanes,
                    sel_bg,
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
    fn render_kitty_row(
        &mut self,
        buf: &mut Buf,
        x_start: u16,
        y: u16,
        avail_w: usize,
        row: &GraphRow,
        is_selected: bool,
        _is_highlighted: bool,
        params: &RenderParams,
        sel_bg: ratatui::style::Color,
    ) {
        let graph_cols = (self.max_lanes as u16) * COLS_PER_LANE;

        render_selection_indicator(
            buf,
            x_start,
            y,
            is_selected,
            sel_bg,
            self.is_active,
            self.palette,
        );

        let img_start = x_start + 1;

        inject_kitty_image(
            buf,
            self.image_cache,
            &row.layout,
            &row.cells,
            params,
            self.palette,
            self.trunk_count,
            img_start,
            x_start,
            y,
            avail_w,
            graph_cols,
            is_selected,
            sel_bg,
        );

        let text_start = img_start.saturating_add(graph_cols).saturating_add(1);
        if text_start >= x_start.saturating_add(avail_w as u16) {
            return;
        }

        let text_budget = (x_start + avail_w as u16).saturating_sub(text_start) as usize;
        let text_spans = build_text_spans(
            row,
            is_selected,
            self.scroll_x,
            text_budget,
            self.trunk_count,
            sel_bg,
            self.branch_index_to_name,
            self.palette,
        );

        let line = Line::from(text_spans);
        buf.set_line(text_start, y, &line, text_budget as u16);

        if is_selected {
            let line_w: usize = line
                .spans
                .iter()
                .map(|s| UnicodeWidthStr::width(s.content.as_ref()))
                .sum();
            let fill_start = text_start.saturating_add(line_w as u16);
            let fill_style = Style::default().bg(sel_bg);
            for x in fill_start..(x_start + avail_w as u16) {
                if let Some(cell) = buf.cell_mut(Position::new(x, y)) {
                    cell.set_style(fill_style);
                }
            }
        }
    }
}

fn render_selection_indicator(
    buf: &mut Buf,
    x_start: u16,
    y: u16,
    is_selected: bool,
    sel_bg: ratatui::style::Color,
    is_active: bool,
    palette: &ThemePalette,
) {
    if is_selected {
        let indicator_fg = if is_active {
            palette.selected_accent
        } else {
            palette.dim_text
        };
        if let Some(cell) = buf.cell_mut(Position::new(x_start, y)) {
            cell.set_symbol("\u{258e}");
            cell.set_style(Style::default().fg(indicator_fg).bg(sel_bg));
        }
    } else if let Some(cell) = buf.cell_mut(Position::new(x_start, y)) {
        cell.set_symbol(" ");
    }
}

#[allow(clippy::too_many_arguments)]
fn inject_kitty_image(
    buf: &mut Buf,
    image_cache: &mut ImageCache,
    layout: &crate::graph::types::RowLayout,
    cells: &[Cell],
    params: &RenderParams,
    palette: &ThemePalette,
    trunk_count: usize,
    img_start: u16,
    x_start: u16,
    y: u16,
    avail_w: usize,
    graph_cols: u16,
    is_selected: bool,
    sel_bg: ratatui::style::Color,
) {
    if let Some(png) = image_cache.get_png(layout, params, palette, trunk_count) {
        let image_id = (y + 1) as u32;
        let encoded = encode_kitty_image(image_id, png, graph_cols, 1);
        if let Some(cell) = buf.cell_mut(Position::new(img_start, y)) {
            cell.set_symbol(&encoded);
            if is_selected {
                cell.set_style(Style::default().bg(sel_bg));
            }
        }
        for col in 1..graph_cols {
            let x = img_start + col;
            if x >= x_start + avail_w as u16 {
                break;
            }
            if let Some(cell) = buf.cell_mut(Position::new(x, y)) {
                cell.set_skip(true);
            }
        }
    } else {
        for (ci, cell_data) in cells.iter().enumerate() {
            let x = img_start + (ci as u16) * 2;
            if x >= x_start + avail_w as u16 {
                break;
            }
            let color = palette.branch_color_by_identity(cell_data.color_index, trunk_count);
            let mut style = Style::default().fg(color);
            if is_selected {
                style = style.bg(sel_bg);
            }
            if let Some(buf_cell) = buf.cell_mut(Position::new(x, y)) {
                buf_cell.set_symbol(cell_glyph(cell_data));
                buf_cell.set_style(style);
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
        Some(row) => &row.layout.lane_branches,
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

use super::truncate_with_ellipsis;

#[allow(clippy::too_many_arguments)]
fn build_row_line(
    row: &GraphRow,
    selected: bool,
    highlighted: bool,
    scroll_x: usize,
    avail_width: usize,
    trunk_count: usize,
    max_lanes: usize,
    sel_bg: ratatui::style::Color,
    branch_index_to_name: &HashMap<usize, String>,
    palette: &ThemePalette,
) -> Line<'static> {
    let mut graph_spans: Vec<Span<'static>> = Vec::new();
    let is_fork = matches!(row.meta.source, CommitSource::Fork(_));

    if selected {
        let indicator_fg = if sel_bg == palette.selected_bg {
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
    // Pad to max_lanes so text starts at a fixed column
    let pad_count = max_lanes.saturating_sub(row.cells.len());
    if pad_count > 0 {
        let pad_style = if selected {
            Style::default().bg(sel_bg)
        } else {
            Style::default()
        };
        graph_spans.push(Span::styled("  ".repeat(pad_count), pad_style));
    }
    graph_spans.push(Span::raw(" "));

    let graph_width: usize = graph_spans
        .iter()
        .map(|s| UnicodeWidthStr::width(s.content.as_ref()))
        .sum();

    let text_budget = avail_width.saturating_sub(graph_width);
    let text_spans = build_text_spans(
        row,
        selected,
        scroll_x,
        text_budget,
        trunk_count,
        sel_bg,
        branch_index_to_name,
        palette,
    );

    let mut spans = graph_spans;
    spans.extend(text_spans);

    Line::from(spans)
}

#[allow(clippy::too_many_arguments)]
fn build_text_spans(
    row: &GraphRow,
    selected: bool,
    scroll_x: usize,
    total_budget: usize,
    trunk_count: usize,
    sel_bg: ratatui::style::Color,
    branch_index_to_name: &HashMap<usize, String>,
    palette: &ThemePalette,
) -> Vec<Span<'static>> {
    let is_fork = matches!(row.meta.source, CommitSource::Fork(_));
    let time_str = format_time_short(&row.meta.time);
    let time_col_w = 5;
    let mut budget = total_budget.saturating_sub(time_col_w);

    let (label_spans, used) =
        branch_label_spans(row, budget, trunk_count, branch_index_to_name, palette);
    budget = budget.saturating_sub(used);

    let mut text_spans = label_spans;

    let author_str = format!(" {}", row.meta.author);
    let author_w = UnicodeWidthStr::width(author_str.as_str());
    let msg_budget = if budget > author_w + 5 {
        budget - author_w
    } else {
        budget
    };

    if msg_budget > 0 {
        let msg = truncate_with_ellipsis(&row.meta.message, msg_budget);
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

    if budget >= author_w && !row.meta.author.is_empty() {
        let style = if selected {
            Style::default().fg(palette.dim_text).bg(sel_bg)
        } else {
            Style::default().fg(palette.dim_text)
        };
        text_spans.push(Span::styled(author_str, style));
        budget = budget.saturating_sub(author_w);
    }

    for name in &row.meta.tag_names {
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

    time_column_spans(
        &mut text_spans,
        &time_str,
        total_budget,
        selected,
        sel_bg,
        palette,
    );

    if scroll_x > 0 {
        text_spans = skip_chars_preserving_style(text_spans, scroll_x);
    }

    text_spans
}

fn branch_label_spans(
    row: &GraphRow,
    mut budget: usize,
    trunk_count: usize,
    branch_index_to_name: &HashMap<usize, String>,
    palette: &ThemePalette,
) -> (Vec<Span<'static>>, usize) {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let initial_budget = budget;
    let commit_color_idx = row.cells.first().map(|c| c.color_index).unwrap_or(0);
    let max_branches = 2;
    let mut showed_branch_label = false;

    for (i, name) in row.meta.branch_names.iter().enumerate() {
        if i >= max_branches || budget < 4 {
            break;
        }
        let max_label = budget.min(20);
        let is_head_branch = row.meta.is_head && i == 0;
        let prefix = if is_head_branch { "*" } else { "" };
        let display = format!("{prefix}{name}");
        let label = truncate_with_ellipsis(&display, max_label.saturating_sub(3));
        let formatted = format!("[{label}] ");
        let w = UnicodeWidthStr::width(formatted.as_str());
        let style = Style::default()
            .fg(palette.branch_color_by_identity(commit_color_idx, trunk_count))
            .add_modifier(Modifier::BOLD);
        spans.push(Span::styled(formatted, style));
        budget = budget.saturating_sub(w);
        showed_branch_label = true;
    }

    let overflow = row.meta.branch_names.len().saturating_sub(max_branches);
    if overflow > 0 && budget >= 5 {
        let overflow_str = format!("[+{overflow}] ");
        let w = UnicodeWidthStr::width(overflow_str.as_str());
        spans.push(Span::styled(
            overflow_str,
            Style::default().fg(palette.dim_text),
        ));
        budget = budget.saturating_sub(w);
        showed_branch_label = true;
    }

    if !showed_branch_label && (row.meta.is_merge || row.meta.is_fork_point) {
        if let Some(bi) = row.meta.branch_index {
            if let Some(name) = branch_index_to_name.get(&bi) {
                let max_label = budget.min(18);
                if max_label >= 4 {
                    let label = truncate_with_ellipsis(name, max_label.saturating_sub(3));
                    let formatted = format!("[{label}] ");
                    let w = UnicodeWidthStr::width(formatted.as_str());
                    let color = palette.branch_color_by_identity(bi, trunk_count);
                    let style = Style::default().fg(color).add_modifier(Modifier::DIM);
                    spans.push(Span::styled(formatted, style));
                    budget = budget.saturating_sub(w);
                }
            }
        }
    }

    (spans, initial_budget - budget)
}

fn time_column_spans(
    text_spans: &mut Vec<Span<'static>>,
    time_str: &str,
    total_budget: usize,
    selected: bool,
    sel_bg: ratatui::style::Color,
    palette: &ThemePalette,
) {
    let current_w: usize = text_spans
        .iter()
        .map(|s| UnicodeWidthStr::width(s.content.as_ref()))
        .sum();
    let remaining = total_budget.saturating_sub(current_w);
    if remaining > time_str.len() {
        let padding = remaining - time_str.len();
        if padding > 0 {
            let pad_style = if selected {
                Style::default().bg(sel_bg)
            } else {
                Style::default()
            };
            text_spans.push(Span::styled(" ".repeat(padding), pad_style));
        }
        let time_style = if selected {
            Style::default().fg(palette.dim_text).bg(sel_bg)
        } else {
            Style::default().fg(palette.dim_text)
        };
        text_spans.push(Span::styled(time_str.to_string(), time_style));
    }
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
