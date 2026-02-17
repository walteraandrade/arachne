use crate::ui::theme;
use ratatui::{
    buffer::Buffer as Buf,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};
use unicode_width::UnicodeWidthStr;

pub struct PaneInfo<'a> {
    pub name: &'a str,
    pub branch: &'a str,
    pub is_active: bool,
    pub commit_count: usize,
}

pub struct HeaderBar<'a> {
    pub panes: &'a [PaneInfo<'a>],
    pub last_sync: &'a str,
    pub author_filter: &'a str,
}

impl<'a> Widget for HeaderBar<'a> {
    fn render(self, area: Rect, buf: &mut Buf) {
        let bg = Style::default().bg(theme::HEADER_BG);
        for x in area.x..area.right() {
            buf[(x, area.y)].set_style(bg);
        }

        let mut spans: Vec<Span<'static>> = Vec::new();

        spans.push(Span::styled(
            " \u{f06f6} arachne",
            Style::default()
                .fg(theme::ACCENT)
                .bg(theme::HEADER_BG)
                .add_modifier(Modifier::BOLD),
        ));

        spans.push(Span::styled(
            " \u{2503} ",
            Style::default().fg(theme::SEPARATOR).bg(theme::HEADER_BG),
        ));

        let single = self.panes.len() == 1;
        if single {
            if let Some(p) = self.panes.first() {
                spans.push(Span::styled(
                    p.name.to_string(),
                    Style::default().bg(theme::HEADER_BG),
                ));
                spans.push(Span::styled(
                    format!(" ({}) ", p.branch),
                    Style::default().fg(theme::ACCENT).bg(theme::HEADER_BG),
                ));
                spans.push(Span::styled(
                    format!("{} commits", p.commit_count),
                    Style::default().fg(theme::DIM_TEXT).bg(theme::HEADER_BG),
                ));
            }
        } else {
            for p in self.panes {
                if p.is_active {
                    spans.push(Span::styled(
                        format!("[{}]", p.name),
                        Style::default()
                            .fg(theme::ACCENT)
                            .bg(theme::HEADER_BG)
                            .add_modifier(Modifier::BOLD),
                    ));
                    spans.push(Span::styled(
                        format!(" ({}) {}  ", p.branch, p.commit_count),
                        Style::default().fg(theme::ACCENT).bg(theme::HEADER_BG),
                    ));
                } else {
                    spans.push(Span::styled(
                        format!("[{}] ", p.name),
                        Style::default().fg(theme::DIM_TEXT).bg(theme::HEADER_BG),
                    ));
                }
            }
        }

        let left_line = Line::from(spans.clone());
        buf.set_line(area.x, area.y, &left_line, area.width);

        // Right zone: sync + help
        let right = format!("synced: {}  ? help ", self.last_sync);
        let right_w = UnicodeWidthStr::width(right.as_str());
        let area_w = area.width as usize;

        // Center zone: author filter indicator
        if !self.author_filter.is_empty() {
            let filter_text = format!("author: {}", self.author_filter);
            let filter_w = UnicodeWidthStr::width(filter_text.as_str());
            let left_used: usize = left_line
                .spans
                .iter()
                .map(|s| UnicodeWidthStr::width(s.content.as_ref()))
                .sum();
            let center_x = (area_w.saturating_sub(filter_w) / 2).max(left_used);
            if center_x + filter_w < area_w.saturating_sub(right_w) {
                let filter_span = Span::styled(
                    filter_text,
                    Style::default()
                        .fg(theme::FILTER_COLOR)
                        .bg(theme::HEADER_BG),
                );
                buf.set_line(
                    area.x + center_x as u16,
                    area.y,
                    &Line::from(filter_span),
                    filter_w as u16,
                );
            }
        }

        if area_w > right_w {
            let right_x = area.x + (area_w - right_w) as u16;
            let right_span = Span::styled(
                right,
                Style::default().fg(theme::DIM_TEXT).bg(theme::HEADER_BG),
            );
            buf.set_line(right_x, area.y, &Line::from(right_span), right_w as u16);
        }
    }
}
