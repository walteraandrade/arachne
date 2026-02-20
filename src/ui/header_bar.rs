use crate::data_source::ViewMode;
use crate::ui::theme::ThemePalette;
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
    pub commit_count: usize,
}

pub struct HeaderBar<'a> {
    pub panes: &'a [PaneInfo<'a>],
    pub last_sync: &'a str,
    pub author_filter: &'a str,
    pub view_mode: Option<&'a ViewMode>,
    pub project_count: usize,
    pub active_project_idx: usize,
    pub palette: &'a ThemePalette,
}

impl<'a> Widget for HeaderBar<'a> {
    fn render(self, area: Rect, buf: &mut Buf) {
        let p = self.palette;
        let bg = Style::default().bg(p.header_bg);
        for x in area.x..area.right() {
            buf[(x, area.y)].set_style(bg);
        }

        let mut spans: Vec<Span<'static>> = Vec::new();

        spans.push(Span::styled(
            " \u{f06f6} arachne",
            Style::default()
                .fg(p.accent)
                .bg(p.header_bg)
                .add_modifier(Modifier::BOLD),
        ));

        spans.push(Span::styled(
            " \u{2503} ",
            Style::default().fg(p.separator).bg(p.header_bg),
        ));

        if let Some(pane) = self.panes.first() {
            spans.push(Span::styled(
                pane.name.to_string(),
                Style::default().bg(p.header_bg),
            ));
            spans.push(Span::styled(
                format!(" ({}) ", pane.branch),
                Style::default().fg(p.accent).bg(p.header_bg),
            ));

            let mode_label = match self.view_mode {
                Some(ViewMode::Local) => "[Local]",
                Some(ViewMode::Remote) => "[Remote]",
                None => "",
            };
            if !mode_label.is_empty() {
                spans.push(Span::styled(
                    format!("{mode_label} "),
                    Style::default().fg(p.dim_text).bg(p.header_bg),
                ));
            }

            spans.push(Span::styled(
                format!("{} commits", pane.commit_count),
                Style::default().fg(p.dim_text).bg(p.header_bg),
            ));

            if self.project_count > 1 {
                spans.push(Span::styled(
                    format!("  [{}/{}]", self.active_project_idx + 1, self.project_count),
                    Style::default().fg(p.dim_text).bg(p.header_bg),
                ));
            }
        }

        let left_line = Line::from(spans.clone());
        buf.set_line(area.x, area.y, &left_line, area.width);

        let right = format!("synced: {}  ? help ", self.last_sync);
        let right_w = UnicodeWidthStr::width(right.as_str());
        let area_w = area.width as usize;

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
                    Style::default().fg(p.filter_color).bg(p.header_bg),
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
            let right_span = Span::styled(right, Style::default().fg(p.dim_text).bg(p.header_bg));
            buf.set_line(right_x, area.y, &Line::from(right_span), right_w as u16);
        }
    }
}
