use crate::ui::input::FilterMode;
use crate::ui::theme;
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
}

impl<'a> Widget for StatusBar<'a> {
    fn render(self, area: Rect, buf: &mut Buf) {
        let bg = Style::default().bg(theme::STATUS_BG);
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
                        .fg(theme::FILTER_COLOR)
                        .bg(theme::STATUS_BG)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(text.to_string(), Style::default().bg(theme::STATUS_BG)),
                Span::styled(
                    "\u{258c}",
                    Style::default()
                        .fg(theme::FILTER_COLOR)
                        .bg(theme::STATUS_BG),
                ),
            ]);
            buf.set_line(area.x, area.y, &line, area.width);
            return;
        }

        let mut left_spans = Vec::new();

        left_spans.push(Span::styled(" ", Style::default().bg(theme::STATUS_BG)));

        // Branch name
        left_spans.push(Span::styled(
            format!("{} ", self.branch_name),
            Style::default().bg(theme::STATUS_BG),
        ));

        // Loading message (replaces sync time when active)
        if let Some(msg) = self.loading_message {
            left_spans.push(Span::styled(
                "\u{2502}",
                Style::default().fg(theme::SEPARATOR).bg(theme::STATUS_BG),
            ));
            left_spans.push(Span::styled(
                format!(" {msg} "),
                Style::default().fg(theme::ACCENT).bg(theme::STATUS_BG),
            ));
        } else {
            // Author filter (if active)
            if !self.author_filter_text.is_empty() {
                left_spans.push(Span::styled(
                    "\u{2502}",
                    Style::default().fg(theme::SEPARATOR).bg(theme::STATUS_BG),
                ));
                left_spans.push(Span::styled(
                    format!(" author: {} ", self.author_filter_text),
                    Style::default()
                        .fg(theme::FILTER_COLOR)
                        .bg(theme::STATUS_BG),
                ));
            }

            // Sync time
            left_spans.push(Span::styled(
                "\u{2502}",
                Style::default().fg(theme::SEPARATOR).bg(theme::STATUS_BG),
            ));
            left_spans.push(Span::styled(
                format!(" synced: {} ", self.last_sync),
                Style::default().bg(theme::STATUS_BG),
            ));
        }

        let left_line = Line::from(left_spans);
        buf.set_line(area.x, area.y, &left_line, area.width);

        // Right zone: keybinding hints with ACCENT keys
        let hint_spans = vec![
            Span::styled("j", Style::default().fg(theme::ACCENT).bg(theme::STATUS_BG)),
            Span::styled("/", Style::default().fg(theme::DIM_TEXT).bg(theme::STATUS_BG)),
            Span::styled("k", Style::default().fg(theme::ACCENT).bg(theme::STATUS_BG)),
            Span::styled(" scroll  ", Style::default().fg(theme::DIM_TEXT).bg(theme::STATUS_BG)),
            Span::styled("/", Style::default().fg(theme::ACCENT).bg(theme::STATUS_BG)),
            Span::styled(" filter  ", Style::default().fg(theme::DIM_TEXT).bg(theme::STATUS_BG)),
            Span::styled("?", Style::default().fg(theme::ACCENT).bg(theme::STATUS_BG)),
            Span::styled(" help ", Style::default().fg(theme::DIM_TEXT).bg(theme::STATUS_BG)),
        ];
        let hints = "j/k scroll  / filter  ? help ";
        let hints_w = UnicodeWidthStr::width(hints);
        let area_w = area.width as usize;

        // Center zone: graph stats
        let stats = format!(
            "{} commits  {} branches",
            self.commit_count, self.branch_count
        );
        let stats_w = UnicodeWidthStr::width(stats.as_str());

        if area_w > hints_w + stats_w {
            // center stats
            let left_used: usize = left_line
                .spans
                .iter()
                .map(|s| UnicodeWidthStr::width(s.content.as_ref()))
                .sum();
            let center_x = (area_w.saturating_sub(stats_w) / 2).max(left_used);
            if center_x + stats_w < area_w.saturating_sub(hints_w) {
                let stats_span = Span::styled(
                    stats,
                    Style::default().fg(theme::DIM_TEXT).bg(theme::STATUS_BG),
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
