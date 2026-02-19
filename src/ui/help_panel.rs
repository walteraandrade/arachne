use crate::ui::theme::ThemePalette;
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
    ("Tab / S-Tab", "Switch project"),
    ("d", "Toggle detail sidebar"),
    ("m", "Toggle Local/Remote"),
    ("Enter", "Detail / Toggle"),
    ("/", "Filter branches"),
    ("a", "Filter author"),
    ("f", "Toggle forks"),
    ("r", "Refresh"),
    ("c", "Config screen"),
    ("?", "This help"),
    ("q / Esc", "Quit / Close"),
];

pub struct HelpPanel<'a> {
    pub palette: &'a ThemePalette,
}

impl<'a> Widget for HelpPanel<'a> {
    fn render(self, area: Rect, buf: &mut Buf) {
        let p = self.palette;
        let popup = super::centered_rect(50, 60, area);
        Clear.render(popup, buf);

        let block = Block::default()
            .title(" Keybindings ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(p.accent));
        let inner = block.inner(popup);
        block.render(popup, buf);

        for (i, (key, desc)) in BINDINGS.iter().enumerate() {
            if i >= inner.height as usize {
                break;
            }
            let y = inner.y + i as u16;
            let key_style = Style::default()
                .fg(p.filter_color)
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
