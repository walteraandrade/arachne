use crate::app::RepoPane;
use crate::git::types::{CommitSource, Oid};
use crate::ui::theme;
use ratatui::{
    buffer::Buffer as Buf,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Widget},
};
use std::collections::HashSet;
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SectionKey {
    Local(usize),
    Remote(usize),
    Fork(usize, String),
    Tags(usize),
}

#[derive(Debug, Clone)]
pub enum EntryKind {
    RepoHeader,
    SectionHeader(SectionKey),
    LocalBranch { is_head: bool, tip: Oid },
    RemoteBranch { tip: Oid },
    ForkBranch { tip: Oid },
    Tag { target: Oid },
}

pub struct DisplayEntry {
    pub label: String,
    pub kind: EntryKind,
}

impl DisplayEntry {
    pub fn tip_oid(&self) -> Option<Oid> {
        match &self.kind {
            EntryKind::LocalBranch { tip, .. } => Some(*tip),
            EntryKind::RemoteBranch { tip } => Some(*tip),
            EntryKind::ForkBranch { tip } => Some(*tip),
            EntryKind::Tag { target } => Some(*target),
            _ => None,
        }
    }

    pub fn section_key(&self) -> Option<&SectionKey> {
        match &self.kind {
            EntryKind::SectionHeader(key) => Some(key),
            _ => None,
        }
    }

    pub fn is_header(&self) -> bool {
        matches!(self.kind, EntryKind::SectionHeader(_) | EntryKind::RepoHeader)
    }
}

pub struct BranchPanel<'a> {
    pub entries: &'a [DisplayEntry],
    pub selected: usize,
    pub scroll: usize,
    pub focused: bool,
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

        let inner_w = inner.width as usize;
        let visible = inner.height as usize;

        for (i, entry) in self.entries.iter().skip(self.scroll).take(visible).enumerate() {
            let y = inner.y + i as u16;
            let abs_idx = self.scroll + i;
            let is_selected = abs_idx == self.selected;

            let line = entry_line(entry, is_selected, inner_w);
            buf.set_line(inner.x, y, &line, inner.width);

            if is_selected {
                for x in inner.x..(inner.x + inner.width) {
                    buf[(x, y)].set_style(Style::default().bg(theme::SELECTED_BG));
                }
            }
        }
    }
}

pub fn build_entries(
    panes: &[RepoPane],
    filter: &str,
    show_forks: bool,
    collapsed: &HashSet<SectionKey>,
) -> Vec<DisplayEntry> {
    let mut entries = Vec::new();

    for (pane_idx, pane) in panes.iter().enumerate() {
        entries.push(DisplayEntry {
            label: pane.repo_name.clone(),
            kind: EntryKind::RepoHeader,
        });

        let branches = &pane.repo_data.branches;
        let tags = &pane.repo_data.tags;

        let local: Vec<_> = branches
            .iter()
            .filter(|b| matches!(b.source, CommitSource::Local) && !b.name.contains('/'))
            .filter(|b| filter.is_empty() || b.name.contains(filter))
            .collect();

        if !local.is_empty() {
            let key = SectionKey::Local(pane_idx);
            let is_collapsed = collapsed.contains(&key);
            entries.push(DisplayEntry {
                label: if is_collapsed {
                    format!("  Local ({}) \u{25b6}", local.len())
                } else {
                    format!("  Local ({}) \u{25bc}", local.len())
                },
                kind: EntryKind::SectionHeader(key),
            });
            if !is_collapsed {
                for b in local {
                    entries.push(DisplayEntry {
                        label: format!(
                            "    {}{}",
                            if b.is_head { "\u{25b8} " } else { "  " },
                            b.name
                        ),
                        kind: EntryKind::LocalBranch { is_head: b.is_head, tip: b.tip },
                    });
                }
            }
        }

        let remote: Vec<_> = branches
            .iter()
            .filter(|b| matches!(b.source, CommitSource::Local | CommitSource::Remote(_)) && b.name.contains('/'))
            .filter(|b| filter.is_empty() || b.name.contains(filter))
            .collect();

        if !remote.is_empty() {
            let key = SectionKey::Remote(pane_idx);
            let is_collapsed = collapsed.contains(&key);
            entries.push(DisplayEntry {
                label: if is_collapsed {
                    format!("  Remote ({}) \u{25b6}", remote.len())
                } else {
                    format!("  Remote ({}) \u{25bc}", remote.len())
                },
                kind: EntryKind::SectionHeader(key),
            });
            if !is_collapsed {
                for b in remote {
                    entries.push(DisplayEntry {
                        label: format!("      {}", b.name),
                        kind: EntryKind::RemoteBranch { tip: b.tip },
                    });
                }
            }
        }

        if show_forks {
            let forks: Vec<_> = branches
                .iter()
                .filter(|b| matches!(b.source, CommitSource::Fork(_)))
                .filter(|b| filter.is_empty() || b.name.contains(filter))
                .collect();

            if !forks.is_empty() {
                let mut current_fork = String::new();
                for b in &forks {
                    if let CommitSource::Fork(ref owner) = b.source {
                        if *owner != current_fork {
                            current_fork = owner.clone();
                            let key = SectionKey::Fork(pane_idx, owner.clone());
                            let fork_count = forks
                                .iter()
                                .filter(|fb| matches!(&fb.source, CommitSource::Fork(o) if o == owner))
                                .count();
                            let is_collapsed = collapsed.contains(&key);
                            entries.push(DisplayEntry {
                                label: if is_collapsed {
                                    format!("  Fork: {owner} ({fork_count}) \u{25b6}")
                                } else {
                                    format!("  Fork: {owner} ({fork_count}) \u{25bc}")
                                },
                                kind: EntryKind::SectionHeader(key),
                            });
                            if !is_collapsed {
                                for fb in &forks {
                                    if matches!(&fb.source, CommitSource::Fork(o) if o == owner) {
                                        entries.push(DisplayEntry {
                                            label: format!("      {}", fb.name),
                                            kind: EntryKind::ForkBranch { tip: fb.tip },
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let filtered_tags: Vec<_> = tags
            .iter()
            .filter(|t| filter.is_empty() || t.name.contains(filter))
            .collect();

        if !filtered_tags.is_empty() {
            let key = SectionKey::Tags(pane_idx);
            let is_collapsed = collapsed.contains(&key);
            entries.push(DisplayEntry {
                label: if is_collapsed {
                    format!("  Tags ({}) \u{25b6}", filtered_tags.len())
                } else {
                    format!("  Tags ({}) \u{25bc}", filtered_tags.len())
                },
                kind: EntryKind::SectionHeader(key),
            });
            if !is_collapsed {
                for t in filtered_tags {
                    entries.push(DisplayEntry {
                        label: format!("    \u{1f3f7} {}", t.name),
                        kind: EntryKind::Tag { target: t.target },
                    });
                }
            }
        }
    }

    entries
}

pub fn max_entry_width(entries: &[DisplayEntry]) -> usize {
    entries
        .iter()
        .map(|e| UnicodeWidthStr::width(e.label.as_str()))
        .max()
        .unwrap_or(15)
}

pub fn auto_collapse_defaults(panes: &[RepoPane], filter: &str) -> HashSet<SectionKey> {
    let mut set = HashSet::new();
    for (pane_idx, pane) in panes.iter().enumerate() {
        let remote_count = pane
            .repo_data
            .branches
            .iter()
            .filter(|b| matches!(b.source, CommitSource::Local | CommitSource::Remote(_)) && b.name.contains('/'))
            .filter(|b| filter.is_empty() || b.name.contains(filter))
            .count();
        if remote_count > 15 {
            set.insert(SectionKey::Remote(pane_idx));
        }
    }
    set
}

fn truncate_left(s: &str, max_width: usize) -> String {
    let w = UnicodeWidthStr::width(s);
    if w <= max_width {
        return s.to_string();
    }
    if max_width <= 1 {
        return "\u{2026}".to_string();
    }
    let target = max_width - 1;
    let mut total_w = 0;
    let mut split_byte = s.len();
    for (byte_idx, ch) in s.char_indices().rev() {
        let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if total_w + cw > target {
            break;
        }
        total_w += cw;
        split_byte = byte_idx;
    }
    format!("\u{2026}{}", &s[split_byte..])
}

fn entry_line(entry: &DisplayEntry, selected: bool, max_width: usize) -> Line<'static> {
    let style = match &entry.kind {
        EntryKind::RepoHeader => Style::default()
            .fg(theme::FILTER_COLOR)
            .add_modifier(Modifier::BOLD),
        EntryKind::SectionHeader(_) => Style::default()
            .fg(ratatui::style::Color::White)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        EntryKind::LocalBranch { is_head: true, .. } => Style::default()
            .fg(theme::HEAD_COLOR)
            .add_modifier(Modifier::BOLD),
        EntryKind::Tag { .. } => Style::default().fg(theme::TAG_COLOR),
        EntryKind::ForkBranch { .. } => Style::default().fg(theme::FORK_DIM),
        _ => Style::default(),
    };

    let style = if selected {
        style.bg(theme::SELECTED_BG)
    } else {
        style
    };

    let label = if !entry.is_header() && max_width > 0 {
        truncate_left(&entry.label, max_width)
    } else {
        entry.label.clone()
    };

    Line::from(Span::styled(label, style))
}
