use super::truncate_with_ellipsis;
use crate::graph::layout::format_time_ago;
use crate::graph::types::RowMeta;
use crate::ui::theme::ThemePalette;
use ratatui::{
    buffer::Buffer as Buf,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Widget,
};

#[allow(dead_code)]
pub struct DetailPanel<'a> {
    pub meta: &'a RowMeta,
    pub focused: bool,
    pub palette: &'a ThemePalette,
}

impl<'a> Widget for DetailPanel<'a> {
    fn render(self, area: Rect, buf: &mut Buf) {
        if area.height < 1 || area.width < 10 {
            return;
        }

        let p = self.palette;
        let inner_y = area.y;
        let inner_w = area.width.saturating_sub(1) as usize;
        let inner_h = area.height as usize;
        if inner_h == 0 || inner_w == 0 {
            return;
        }

        let label_style = Style::default().fg(p.accent);
        let mut y = inner_y;
        let x = area.x + 1;

        if (y - inner_y) as usize >= inner_h {
            return;
        }
        let sha = &self.meta.oid.to_string()[..8.min(self.meta.oid.to_string().len())];
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

        if (y - inner_y) as usize >= inner_h {
            return;
        }
        buf.set_line(
            x,
            y,
            &Line::from(vec![
                Span::styled("Author ", label_style),
                Span::raw(self.meta.author.as_str()),
            ]),
            inner_w as u16,
        );
        y += 1;

        if (y - inner_y) as usize >= inner_h {
            return;
        }
        let time_ago = format_time_ago(&self.meta.time);
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

        if !self.meta.branch_names.is_empty() && ((y - inner_y) as usize) < inner_h {
            buf.set_line(
                x,
                y,
                &Line::from(vec![
                    Span::styled("Refs ", label_style),
                    Span::raw(self.meta.branch_names.join(", ")),
                ]),
                inner_w as u16,
            );
            y += 1;
        }

        if !self.meta.tag_names.is_empty() && ((y - inner_y) as usize) < inner_h {
            buf.set_line(
                x,
                y,
                &Line::from(vec![
                    Span::styled("Tags ", label_style),
                    Span::styled(
                        self.meta.tag_names.join(", "),
                        Style::default().fg(p.tag_color),
                    ),
                ]),
                inner_w as u16,
            );
            y += 1;
        }

        if ((y - inner_y) as usize) < inner_h {
            y += 1;
        }
        let remaining = inner_h.saturating_sub((y - inner_y) as usize);
        if remaining > 0 {
            for (i, line) in self.meta.message.lines().enumerate() {
                if i >= remaining {
                    break;
                }
                let truncated = truncate_with_ellipsis(line, inner_w);
                buf.set_line(
                    x,
                    y + i as u16,
                    &Line::from(Span::raw(truncated)),
                    inner_w as u16,
                );
            }
        }
    }
}
