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

        let mut left_spans = Vec::new();

        left_spans.push(Span::styled(" ", Style::default().bg(theme::STATUS_BG)));

        // Branch name
        left_spans.push(Span::styled(
            format!("{} ", self.branch_name),
            Style::default().bg(theme::STATUS_BG),
        ));

        // Author filter (if active)
        if !self.author_filter_text.is_empty() {
            left_spans.push(Span::styled(
                "\u{2502}",
                Style::default()
                    .fg(theme::SEPARATOR)
                    .bg(theme::STATUS_BG),
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
            Style::default()
                .fg(theme::SEPARATOR)
                .bg(theme::STATUS_BG),
        ));
        left_spans.push(Span::styled(
            format!(" synced: {} ", self.last_sync),
            Style::default().bg(theme::STATUS_BG),
        ));

        let left_line = Line::from(left_spans);
        buf.set_line(area.x, area.y, &left_line, area.width);

        // Right-aligned "? help"
        let help_text = "? help ";
        let help_w = UnicodeWidthStr::width(help_text);
        let area_w = area.width as usize;
        if area_w > help_w {
            let help_x = area.x + (area_w - help_w) as u16;
            let help_span = Span::styled(
                help_text,
                Style::default().fg(theme::DIM_TEXT).bg(theme::STATUS_BG),
            );
            buf.set_line(help_x, area.y, &Line::from(help_span), help_w as u16);
        }
    }
}
