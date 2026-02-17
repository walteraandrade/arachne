use crate::graph::layout::format_time_ago;
use crate::graph::types::GraphRow;
use crate::ui::theme;
use ratatui::{
    buffer::Buffer as Buf,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};

pub struct DetailPanel<'a> {
    pub row: &'a GraphRow,
    pub focused: bool,
}

impl<'a> Widget for DetailPanel<'a> {
    fn render(self, area: Rect, buf: &mut Buf) {
        if area.height < 3 || area.width < 10 {
            return;
        }

        // Header line
        let header_fg = if self.focused {
            theme::ACCENT
        } else {
            theme::PANEL_LABEL
        };
        let header_style = Style::default()
            .fg(header_fg)
            .bg(theme::HEADER_BG)
            .add_modifier(Modifier::BOLD);
        for x in area.x..area.right() {
            buf[(x, area.y)].set_style(Style::default().bg(theme::HEADER_BG));
        }
        buf.set_line(
            area.x,
            area.y,
            &Line::from(Span::styled(" Detail", header_style)),
            area.width,
        );

        // Separator
        let sep_style = Style::default().fg(theme::SEPARATOR);
        for x in area.x..area.right() {
            buf[(x, area.y + 1)].set_char('\u{2500}');
            buf[(x, area.y + 1)].set_style(sep_style);
        }

        let inner_y = area.y + 2;
        let inner_w = area.width.saturating_sub(1) as usize;
        let inner_h = area.height.saturating_sub(2) as usize;
        if inner_h == 0 || inner_w == 0 {
            return;
        }

        let label_style = Style::default().fg(theme::DIM_TEXT);
        let mut y = inner_y;
        let x = area.x + 1;

        // SHA
        if (y - inner_y) as usize >= inner_h {
            return;
        }
        let sha = &self.row.oid.to_string()[..8.min(self.row.oid.to_string().len())];
        buf.set_line(
            x,
            y,
            &Line::from(vec![
                Span::styled("SHA ", label_style),
                Span::raw(sha.to_string()),
            ]),
            inner_w as u16,
        );
        y += 1;

        // Author
        if (y - inner_y) as usize >= inner_h {
            return;
        }
        buf.set_line(
            x,
            y,
            &Line::from(vec![
                Span::styled("Author ", label_style),
                Span::raw(self.row.author.as_str()),
            ]),
            inner_w as u16,
        );
        y += 1;

        // Time
        if (y - inner_y) as usize >= inner_h {
            return;
        }
        let time_ago = format_time_ago(&self.row.time);
        buf.set_line(
            x,
            y,
            &Line::from(vec![
                Span::styled("Time ", label_style),
                Span::raw(time_ago),
            ]),
            inner_w as u16,
        );
        y += 1;

        // Branches
        if !self.row.branch_names.is_empty() && ((y - inner_y) as usize) < inner_h {
            buf.set_line(
                x,
                y,
                &Line::from(vec![
                    Span::styled("Refs ", label_style),
                    Span::raw(self.row.branch_names.join(", ")),
                ]),
                inner_w as u16,
            );
            y += 1;
        }

        // Tags
        if !self.row.tag_names.is_empty() && ((y - inner_y) as usize) < inner_h {
            buf.set_line(
                x,
                y,
                &Line::from(vec![
                    Span::styled("Tags ", label_style),
                    Span::styled(
                        self.row.tag_names.join(", "),
                        Style::default().fg(theme::TAG_COLOR),
                    ),
                ]),
                inner_w as u16,
            );
            y += 1;
        }

        // Blank line + message
        if ((y - inner_y) as usize) < inner_h {
            y += 1;
        }
        let remaining = inner_h.saturating_sub((y - inner_y) as usize);
        if remaining > 0 {
            for (i, line) in self.row.message.lines().enumerate() {
                if i >= remaining {
                    break;
                }
                let truncated = if line.len() > inner_w {
                    &line[..inner_w]
                } else {
                    line
                };
                buf.set_line(
                    x,
                    y + i as u16,
                    &Line::from(Span::raw(truncated.to_string())),
                    inner_w as u16,
                );
            }
        }
    }
}
