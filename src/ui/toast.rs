use crate::ui::theme::ThemePalette;
use ratatui::{
    buffer::Buffer as Buf,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Widget},
};
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone)]
pub enum NotifyLevel {
    Error,
    Warn,
    #[allow(dead_code)]
    Info,
}

impl NotifyLevel {
    pub fn ttl_secs(&self) -> u64 {
        match self {
            NotifyLevel::Error => 30,
            NotifyLevel::Warn => 8,
            NotifyLevel::Info => 5,
        }
    }

    pub fn color(&self, palette: &ThemePalette) -> Color {
        match self {
            NotifyLevel::Error => palette.error_fg,
            NotifyLevel::Warn => palette.warn_fg,
            NotifyLevel::Info => palette.accent,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Notification {
    pub message: String,
    pub level: NotifyLevel,
    pub created: std::time::Instant,
}

pub struct Toast<'a> {
    pub notification: &'a Notification,
    pub palette: &'a ThemePalette,
}

impl<'a> Widget for Toast<'a> {
    fn render(self, area: Rect, buf: &mut Buf) {
        let first_line = self.notification.message.lines().next().unwrap_or("");
        let text_w = UnicodeWidthStr::width(first_line);
        let box_w = text_w.saturating_add(4).min(area.width as usize) as u16;
        let box_h: u16 = 3;

        if area.width < box_w || area.height < box_h.saturating_add(1) {
            return;
        }

        let x = area.right().saturating_sub(box_w.saturating_add(1));
        let y = area.bottom().saturating_sub(box_h + 1);
        let toast_area = Rect::new(x, y, box_w, box_h);

        Clear.render(toast_area, buf);

        let color = self.notification.level.color(self.palette);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(color));
        let inner = block.inner(toast_area);
        block.render(toast_area, buf);

        if inner.width == 0 {
            return;
        }

        let truncated: String = if text_w > inner.width as usize {
            first_line
                .chars()
                .take((inner.width as usize).saturating_sub(1))
                .collect::<String>()
                + "\u{2026}"
        } else {
            first_line.to_string()
        };

        let line = Line::from(Span::styled(truncated, Style::default().fg(color)));
        buf.set_line(inner.x, inner.y, &line, inner.width);
    }
}
