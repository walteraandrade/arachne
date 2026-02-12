use crate::app::RepoPane;
use crate::git::types::CommitSource;
use crate::ui::theme;
use ratatui::{
    buffer::Buffer as Buf,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Widget},
};

pub struct BranchPanel<'a> {
    pub panes: &'a [RepoPane],
    pub selected: usize,
    pub scroll: usize,
    pub filter: &'a str,
    pub focused: bool,
    pub show_forks: bool,
}

struct DisplayEntry {
    label: String,
    is_head: bool,
    is_tag: bool,
    is_fork: bool,
    is_header: bool,
    is_repo_header: bool,
}

impl<'a> Widget for BranchPanel<'a> {
    fn render(self, area: Rect, buf: &mut Buf) {
        let border_style = if self.focused {
            Style::default().fg(theme::FILTER_COLOR)
        } else {
            Style::default().fg(theme::BORDER_COLOR)
        };

        let block = Block::default()
            .title(" Branches ")
            .borders(Borders::ALL)
            .border_style(border_style);
        let inner = block.inner(area);
        block.render(area, buf);

        let entries = self.build_entries();
        let visible = inner.height as usize;

        for (i, entry) in entries.iter().skip(self.scroll).take(visible).enumerate() {
            let y = inner.y + i as u16;
            let abs_idx = self.scroll + i;
            let is_selected = abs_idx == self.selected;

            let line = entry_line(entry, is_selected);
            buf.set_line(inner.x, y, &line, inner.width);

            if is_selected {
                for x in inner.x..(inner.x + inner.width) {
                    buf[(x, y)].set_style(Style::default().bg(theme::SELECTED_BG));
                }
            }
        }
    }
}

impl<'a> BranchPanel<'a> {
    fn build_entries(&self) -> Vec<DisplayEntry> {
        let mut entries = Vec::new();

        for pane in self.panes {
            // Repo-level header
            entries.push(DisplayEntry {
                label: pane.repo_name.clone(),
                is_head: false,
                is_tag: false,
                is_fork: false,
                is_header: false,
                is_repo_header: true,
            });

            let branches = &pane.repo_data.branches;
            let tags = &pane.repo_data.tags;

            let local: Vec<_> = branches
                .iter()
                .filter(|b| matches!(b.source, CommitSource::Local) && !b.name.contains('/'))
                .filter(|b| self.filter.is_empty() || b.name.contains(self.filter))
                .collect();

            if !local.is_empty() {
                entries.push(DisplayEntry {
                    label: "  Local".to_string(),
                    is_head: false,
                    is_tag: false,
                    is_fork: false,
                    is_header: true,
                    is_repo_header: false,
                });
                for b in local {
                    entries.push(DisplayEntry {
                        label: format!(
                            "    {}{}",
                            if b.is_head { "▸ " } else { "  " },
                            b.name
                        ),
                        is_head: b.is_head,
                        is_tag: false,
                        is_fork: false,
                        is_header: false,
                        is_repo_header: false,
                    });
                }
            }

            let remote: Vec<_> = branches
                .iter()
                .filter(|b| matches!(b.source, CommitSource::Local) && b.name.contains('/'))
                .filter(|b| self.filter.is_empty() || b.name.contains(self.filter))
                .collect();

            if !remote.is_empty() {
                entries.push(DisplayEntry {
                    label: "  Remote".to_string(),
                    is_head: false,
                    is_tag: false,
                    is_fork: false,
                    is_header: true,
                    is_repo_header: false,
                });
                for b in remote {
                    entries.push(DisplayEntry {
                        label: format!("      {}", b.name),
                        is_head: false,
                        is_tag: false,
                        is_fork: false,
                        is_header: false,
                        is_repo_header: false,
                    });
                }
            }

            if self.show_forks {
                let forks: Vec<_> = branches
                    .iter()
                    .filter(|b| matches!(b.source, CommitSource::Fork(_)))
                    .filter(|b| self.filter.is_empty() || b.name.contains(self.filter))
                    .collect();

                if !forks.is_empty() {
                    let mut current_fork = String::new();
                    for b in forks {
                        if let CommitSource::Fork(ref owner) = b.source {
                            if *owner != current_fork {
                                current_fork = owner.clone();
                                entries.push(DisplayEntry {
                                    label: format!("  Fork: {owner}"),
                                    is_head: false,
                                    is_tag: false,
                                    is_fork: true,
                                    is_header: true,
                                    is_repo_header: false,
                                });
                            }
                        }
                        entries.push(DisplayEntry {
                            label: format!("      {}", b.name),
                            is_head: false,
                            is_tag: false,
                            is_fork: true,
                            is_header: false,
                            is_repo_header: false,
                        });
                    }
                }
            }

            let filtered_tags: Vec<_> = tags
                .iter()
                .filter(|t| self.filter.is_empty() || t.name.contains(self.filter))
                .collect();

            if !filtered_tags.is_empty() {
                entries.push(DisplayEntry {
                    label: "  Tags".to_string(),
                    is_head: false,
                    is_tag: true,
                    is_fork: false,
                    is_header: true,
                    is_repo_header: false,
                });
                for t in filtered_tags {
                    entries.push(DisplayEntry {
                        label: format!("    🏷 {}", t.name),
                        is_head: false,
                        is_tag: true,
                        is_fork: false,
                        is_header: false,
                        is_repo_header: false,
                    });
                }
            }
        }

        entries
    }

    pub fn entry_count(&self) -> usize {
        self.build_entries().len()
    }
}

fn entry_line(entry: &DisplayEntry, selected: bool) -> Line<'static> {
    let style = if entry.is_repo_header {
        Style::default()
            .fg(theme::FILTER_COLOR)
            .add_modifier(Modifier::BOLD)
    } else if entry.is_header {
        Style::default()
            .fg(ratatui::style::Color::White)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    } else if entry.is_head {
        Style::default()
            .fg(theme::HEAD_COLOR)
            .add_modifier(Modifier::BOLD)
    } else if entry.is_tag {
        Style::default().fg(theme::TAG_COLOR)
    } else if entry.is_fork {
        Style::default().fg(theme::FORK_DIM)
    } else {
        Style::default()
    };

    let style = if selected {
        style.bg(theme::SELECTED_BG)
    } else {
        style
    };

    Line::from(Span::styled(entry.label.clone(), style))
}
