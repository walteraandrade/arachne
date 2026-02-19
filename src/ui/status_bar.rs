use crate::ui::input::FilterMode;
use crate::ui::theme::ThemePalette;
use ratatui::{
    buffer::Buffer as Buf,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};
use unicode_width::UnicodeWidthStr;

pub struct StatusBar<'a> {
    pub branch_name: &'a str,
    pub last_sync: &'a str,
    pub filter_mode: FilterMode,
    pub filter_text: &'a str,
    pub author_filter_text: &'a str,
    pub loading_message: Option<&'a str>,
    pub commit_count: usize,
    pub branch_count: usize,
    pub palette: &'a ThemePalette,
}

impl<'a> Widget for StatusBar<'a> {
    fn render(self, area: Rect, buf: &mut Buf) {
        let p = self.palette;
        let bg = Style::default().bg(p.status_bg);
        for x in area.x..area.right() {
            buf[(x, area.y)].set_style(bg);
        }

        if self.filter_mode.is_active() {
            let (prefix, text) = match self.filter_mode {
                FilterMode::Branch => (" /", self.filter_text),
                FilterMode::Author => (" a/", self.author_filter_text),
                FilterMode::Off => return,
            };
            let line = Line::from(vec![
                Span::styled(
                    prefix,
                    Style::default()
                        .fg(p.filter_color)
                        .bg(p.status_bg)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(text.to_string(), Style::default().bg(p.status_bg)),
                Span::styled(
                    "\u{258c}",
                    Style::default().fg(p.filter_color).bg(p.status_bg),
                ),
            ]);
            buf.set_line(area.x, area.y, &line, area.width);
            return;
        }

        let mut left_spans = Vec::new();

        left_spans.push(Span::styled(" ", Style::default().bg(p.status_bg)));

        left_spans.push(Span::styled(
            format!("{} ", self.branch_name),
            Style::default().bg(p.status_bg),
        ));

        if let Some(msg) = self.loading_message {
            left_spans.push(Span::styled(
                "\u{2502}",
                Style::default().fg(p.separator).bg(p.status_bg),
            ));
            left_spans.push(Span::styled(
                format!(" {msg} "),
                Style::default().fg(p.accent).bg(p.status_bg),
            ));
        } else {
            if !self.author_filter_text.is_empty() {
                left_spans.push(Span::styled(
                    "\u{2502}",
                    Style::default().fg(p.separator).bg(p.status_bg),
                ));
                left_spans.push(Span::styled(
                    format!(" author: {} ", self.author_filter_text),
                    Style::default().fg(p.filter_color).bg(p.status_bg),
                ));
            }

            left_spans.push(Span::styled(
                "\u{2502}",
                Style::default().fg(p.separator).bg(p.status_bg),
            ));
            left_spans.push(Span::styled(
                format!(" synced: {} ", self.last_sync),
                Style::default().bg(p.status_bg),
            ));
        }

        let left_line = Line::from(left_spans);
        buf.set_line(area.x, area.y, &left_line, area.width);

        let hint_spans = vec![
            Span::styled("j", Style::default().fg(p.accent).bg(p.status_bg)),
            Span::styled("/", Style::default().fg(p.dim_text).bg(p.status_bg)),
            Span::styled("k", Style::default().fg(p.accent).bg(p.status_bg)),
            Span::styled(" scroll  ", Style::default().fg(p.dim_text).bg(p.status_bg)),
            Span::styled("/", Style::default().fg(p.accent).bg(p.status_bg)),
            Span::styled(" filter  ", Style::default().fg(p.dim_text).bg(p.status_bg)),
            Span::styled("?", Style::default().fg(p.accent).bg(p.status_bg)),
            Span::styled(" help ", Style::default().fg(p.dim_text).bg(p.status_bg)),
        ];
        let hints = "j/k scroll  / filter  ? help ";
        let hints_w = UnicodeWidthStr::width(hints);
        let area_w = area.width as usize;

        let stats = format!(
            "{} commits  {} branches",
            self.commit_count, self.branch_count
        );
        let stats_w = UnicodeWidthStr::width(stats.as_str());

        if area_w > hints_w + stats_w {
            let left_used: usize = left_line
                .spans
                .iter()
                .map(|s| UnicodeWidthStr::width(s.content.as_ref()))
                .sum();
            let center_x = (area_w.saturating_sub(stats_w) / 2).max(left_used);
            if center_x + stats_w < area_w.saturating_sub(hints_w) {
                let stats_span = Span::styled(
                    stats,
                    Style::default().fg(p.dim_text).bg(p.status_bg),
                );
                buf.set_line(
                    area.x + center_x as u16,
                    area.y,
                    &Line::from(stats_span),
                    stats_w as u16,
                );
            }

            let hints_x = area.x + (area_w - hints_w) as u16;
            let hints_line = Line::from(hint_spans.clone());
            buf.set_line(hints_x, area.y, &hints_line, hints_w as u16);
        } else if area_w > hints_w {
            let hints_x = area.x + (area_w - hints_w) as u16;
            let hints_line = Line::from(hint_spans);
            buf.set_line(hints_x, area.y, &hints_line, hints_w as u16);
        }
    }
}
