use crate::ui::theme;
use ratatui::{
    buffer::Buffer as Buf,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Widget},
};

const BINDINGS: &[(&str, &str)] = &[
    ("j/k  \u{2191}/\u{2193}", "Scroll"),
    ("h/l  \u{2190}/\u{2192}", "Switch panel"),
    ("H/L", "Scroll text"),
    ("Tab / S-Tab", "Switch pane"),
    ("Enter", "Detail / Toggle"),
    ("/", "Filter branches"),
    ("a", "Filter author"),
    ("f", "Toggle forks"),
    ("r", "Refresh"),
    ("?", "This help"),
    ("q", "Quit"),
];

pub struct HelpPanel;

impl Widget for HelpPanel {
    fn render(self, area: Rect, buf: &mut Buf) {
        let popup = super::centered_rect(50, 60, area);
        Clear.render(popup, buf);

        let block = Block::default()
            .title(" Keybindings ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::ACCENT));
        let inner = block.inner(popup);
        block.render(popup, buf);

        for (i, (key, desc)) in BINDINGS.iter().enumerate() {
            if i >= inner.height as usize {
                break;
            }
            let y = inner.y + i as u16;
            let key_style = Style::default()
                .fg(theme::FILTER_COLOR)
                .add_modifier(Modifier::BOLD);
            let desc_style = Style::default();

            let key_col_w = 16;
            let line = Line::from(vec![
                Span::styled(format!(" {:<width$}", key, width = key_col_w), key_style),
                Span::styled(desc.to_string(), desc_style),
            ]);
            buf.set_line(inner.x, y, &line, inner.width);
        }
    }
}
