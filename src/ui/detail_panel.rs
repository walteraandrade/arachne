use crate::graph::types::GraphRow;
use crate::ui::theme;
use ratatui::{
    buffer::Buffer as Buf,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap},
};

pub struct DetailPanel<'a> {
    pub row: &'a GraphRow,
}

impl<'a> Widget for DetailPanel<'a> {
    fn render(self, area: Rect, buf: &mut Buf) {
        let popup = centered_rect(60, 40, area);
        Clear.render(popup, buf);

        let block = Block::default()
            .title(" Commit Detail ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::FILTER_COLOR));
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

        let oid_line = Line::from(vec![
            Span::styled("SHA: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(self.row.oid.to_string()),
        ]);
        buf.set_line(chunks[0].x, chunks[0].y, &oid_line, chunks[0].width);

        let author_line = Line::from(vec![
            Span::styled("Author: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(self.row.author.clone()),
        ]);
        buf.set_line(chunks[1].x, chunks[1].y, &author_line, chunks[1].width);

        let time_line = Line::from(vec![
            Span::styled("Time: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(self.row.time_ago.clone()),
        ]);
        buf.set_line(chunks[2].x, chunks[2].y, &time_line, chunks[2].width);

        if !self.row.branch_names.is_empty() {
            let branch_line = Line::from(vec![
                Span::styled("Branches: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(self.row.branch_names.join(", ")),
            ]);
            buf.set_line(chunks[3].x, chunks[3].y, &branch_line, chunks[3].width);
        }

        if !self.row.tag_names.is_empty() {
            let tag_line = Line::from(vec![
                Span::styled("Tags: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(
                    self.row.tag_names.join(", "),
                    Style::default().fg(theme::TAG_COLOR),
                ),
            ]);
            buf.set_line(chunks[4].x, chunks[4].y, &tag_line, chunks[4].width);
        }

        let msg = Paragraph::new(self.row.message.clone()).wrap(Wrap { trim: true });
        msg.render(chunks[5], buf);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vert = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(area);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(vert[1])[1]
}
