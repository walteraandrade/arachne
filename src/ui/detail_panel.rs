use crate::graph::layout::format_time_ago;
use crate::graph::types::GraphRow;
use crate::ui::theme;
use ratatui::{
    buffer::Buffer as Buf,
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap},
};

pub struct DetailPanel<'a> {
    pub row: &'a GraphRow,
}

impl<'a> Widget for DetailPanel<'a> {
    fn render(self, area: Rect, buf: &mut Buf) {
        let popup = super::centered_rect(70, 50, area);
        Clear.render(popup, buf);

        let block = Block::default()
            .title(" Commit Detail ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::ACCENT));
        let inner = block.inner(popup);
        block.render(popup, buf);

        let chunks = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .split(inner);

        let label_style = Style::default().fg(theme::DIM_TEXT);

        let short_sha = &self.row.oid.to_string()[..8.min(self.row.oid.to_string().len())];
        let oid_line = Line::from(vec![
            Span::styled("SHA: ", label_style),
            Span::raw(short_sha.to_string()),
        ]);
        buf.set_line(chunks[0].x, chunks[0].y, &oid_line, chunks[0].width);

        let author_line = Line::from(vec![
            Span::styled("Author: ", label_style),
            Span::raw(self.row.author.as_str()),
        ]);
        buf.set_line(chunks[1].x, chunks[1].y, &author_line, chunks[1].width);

        let time_ago = format_time_ago(&self.row.time);
        let time_line = Line::from(vec![
            Span::styled("Time: ", label_style),
            Span::raw(time_ago),
        ]);
        buf.set_line(chunks[2].x, chunks[2].y, &time_line, chunks[2].width);

        if !self.row.branch_names.is_empty() {
            let branch_line = Line::from(vec![
                Span::styled("Branches: ", label_style),
                Span::raw(self.row.branch_names.join(", ")),
            ]);
            buf.set_line(chunks[3].x, chunks[3].y, &branch_line, chunks[3].width);
        }

        if !self.row.tag_names.is_empty() {
            let tag_line = Line::from(vec![
                Span::styled("Tags: ", label_style),
                Span::styled(
                    self.row.tag_names.join(", "),
                    Style::default().fg(theme::TAG_COLOR),
                ),
            ]);
            buf.set_line(chunks[4].x, chunks[4].y, &tag_line, chunks[4].width);
        }

        let msg = Paragraph::new(self.row.message.as_str()).wrap(Wrap { trim: true });
        msg.render(chunks[5], buf);
    }
}
