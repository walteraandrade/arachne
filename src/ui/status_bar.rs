use crate::ui::input::FilterMode;
use crate::ui::theme;
use ratatui::{
    buffer::Buffer as Buf,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};

pub struct StatusBar<'a> {
    pub pane_tabs: &'a [(&'a str, bool)],
    pub branch_name: &'a str,
    pub last_sync: &'a str,
    pub rate_limit: Option<u32>,
    pub filter_mode: FilterMode,
    pub filter_text: &'a str,
    pub author_filter_text: &'a str,
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
                FilterMode::Off => unreachable!(),
            };
            let line = Line::from(vec![
                Span::styled(
                    prefix,
                    Style::default()
                        .fg(theme::FILTER_COLOR)
                        .bg(theme::STATUS_BG)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    text.to_string(),
                    Style::default().bg(theme::STATUS_BG),
                ),
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

        let mut spans = Vec::new();

        spans.push(Span::styled(" ", Style::default().bg(theme::STATUS_BG)));
        for (name, is_active) in self.pane_tabs {
            let style = if *is_active {
                Style::default()
                    .fg(theme::FILTER_COLOR)
                    .bg(theme::STATUS_BG)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(theme::BORDER_COLOR)
                    .bg(theme::STATUS_BG)
            };
            let short = name.rsplit('/').next().unwrap_or(name);
            spans.push(Span::styled(format!("[{short}]"), style));
            spans.push(Span::styled(" ", Style::default().bg(theme::STATUS_BG)));
        }

        spans.push(Span::styled(
            "\u{2502}",
            Style::default()
                .fg(theme::BORDER_COLOR)
                .bg(theme::STATUS_BG),
        ));
        spans.push(Span::styled(
            format!(" {} ", self.branch_name),
            Style::default().bg(theme::STATUS_BG),
        ));

        if !self.author_filter_text.is_empty() {
            spans.push(Span::styled(
                "\u{2502}",
                Style::default()
                    .fg(theme::BORDER_COLOR)
                    .bg(theme::STATUS_BG),
            ));
            spans.push(Span::styled(
                format!(" author: {} ", self.author_filter_text),
                Style::default()
                    .fg(theme::FILTER_COLOR)
                    .bg(theme::STATUS_BG),
            ));
        }

        spans.push(Span::styled(
            "\u{2502}",
            Style::default()
                .fg(theme::BORDER_COLOR)
                .bg(theme::STATUS_BG),
        ));
        spans.push(Span::styled(
            format!(" synced: {} ", self.last_sync),
            Style::default().bg(theme::STATUS_BG),
        ));

        if let Some(remaining) = self.rate_limit {
            spans.push(Span::styled(
                "\u{2502}",
                Style::default()
                    .fg(theme::BORDER_COLOR)
                    .bg(theme::STATUS_BG),
            ));
            spans.push(Span::styled(
                format!(" API: {remaining} "),
                Style::default().bg(theme::STATUS_BG),
            ));
        }

        let line = Line::from(spans);
        buf.set_line(area.x, area.y, &line, area.width);
    }
}
