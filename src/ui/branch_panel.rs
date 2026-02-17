use crate::git::types::{CommitSource, Oid};
use crate::project::Project;
use crate::graph::branch_assign::strip_remote_prefix;
use crate::ui::theme;
use ratatui::{
    buffer::Buffer as Buf,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
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
    SectionHeader { key: SectionKey, count: usize },
    Spacer,
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
            EntryKind::SectionHeader { ref key, .. } => Some(key),
            _ => None,
        }
    }

    pub fn is_header(&self) -> bool {
        matches!(
            self.kind,
            EntryKind::SectionHeader { .. } | EntryKind::RepoHeader
        )
    }

    pub fn is_spacer(&self) -> bool {
        matches!(self.kind, EntryKind::Spacer)
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
        if area.height < 2 {
            return;
        }

        // Header line: "Branches"
        let header_fg = if self.focused {
            theme::ACCENT
        } else {
            theme::PANEL_LABEL
        };
        let header_style = Style::default()
            .fg(header_fg)
            .bg(theme::HEADER_BG)
            .add_modifier(Modifier::BOLD);
        for x in area.x..area.right() {
            buf[(x, area.y)].set_style(Style::default().bg(theme::HEADER_BG));
        }
        buf.set_line(
            area.x,
            area.y,
            &Line::from(Span::styled("Branches", header_style)),
            area.width,
        );

        // Horizontal separator
        if area.height < 3 {
            return;
        }
        let sep_style = Style::default().fg(theme::SEPARATOR);
        for x in area.x..area.right() {
            buf[(x, area.y + 1)].set_char('\u{2500}');
            buf[(x, area.y + 1)].set_style(sep_style);
        }

        let inner_y = area.y + 2;
        let inner_w = area.width as usize;
        let visible = (area.height.saturating_sub(2)) as usize;

        let sel_bg = if self.focused {
            theme::SELECTED_BG
        } else {
            theme::UNFOCUSED_SEL_BG
        };

        for (i, entry) in self
            .entries
            .iter()
            .skip(self.scroll)
            .take(visible)
            .enumerate()
        {
            let y = inner_y + i as u16;
            let abs_idx = self.scroll + i;
            let is_selected = abs_idx == self.selected;

            let line = entry_line(entry, is_selected, inner_w, self.focused);
            buf.set_line(area.x, y, &line, area.width);

            if is_selected {
                for x in area.x..(area.x + area.width) {
                    buf[(x, y)].set_style(Style::default().bg(sel_bg));
                }
            }
        }
    }
}

pub fn build_entries(
    projects: &[Project],
    filter: &str,
    show_forks: bool,
    collapsed: &HashSet<SectionKey>,
) -> Vec<DisplayEntry> {
    let mut entries = Vec::new();
    let single_pane = projects.len() == 1;

    for (project_idx, proj) in projects.iter().enumerate() {
        if !single_pane {
            entries.push(DisplayEntry {
                label: proj.name.clone(),
                kind: EntryKind::RepoHeader,
            });
        }

        let branches = &proj.repo_data.branches;
        let tags = &proj.repo_data.tags;

        let local: Vec<_> = branches
            .iter()
            .filter(|b| matches!(b.source, CommitSource::Local) && !b.name.contains('/'))
            .filter(|b| filter.is_empty() || b.name.contains(filter))
            .collect();

        let has_local = !local.is_empty();
        if has_local {
            let key = SectionKey::Local(project_idx);
            let is_collapsed = collapsed.contains(&key);
            let arrow = if is_collapsed { "\u{25b6}" } else { "\u{25bc}" };
            let count = local.len();
            entries.push(DisplayEntry {
                label: format!("  {arrow} Local"),
                kind: EntryKind::SectionHeader { key, count },
            });
            if !is_collapsed {
                for b in local {
                    entries.push(DisplayEntry {
                        label: format!(
                            "    {}{}",
                            if b.is_head { "\u{25b8} " } else { "  " },
                            b.name
                        ),
                        kind: EntryKind::LocalBranch {
                            is_head: b.is_head,
                            tip: b.tip,
                        },
                    });
                }
            }
        }

        let remote: Vec<_> = branches
            .iter()
            .filter(|b| {
                matches!(b.source, CommitSource::Local | CommitSource::Remote(_))
                    && b.name.contains('/')
            })
            .filter(|b| filter.is_empty() || b.name.contains(filter))
            .collect();

        if !remote.is_empty() {
            if has_local {
                entries.push(DisplayEntry {
                    label: String::new(),
                    kind: EntryKind::Spacer,
                });
            }
            let key = SectionKey::Remote(project_idx);
            let is_collapsed = collapsed.contains(&key);
            let arrow = if is_collapsed { "\u{25b6}" } else { "\u{25bc}" };
            let count = remote.len();
            entries.push(DisplayEntry {
                label: format!("  {arrow} Remote"),
                kind: EntryKind::SectionHeader { key, count },
            });
            if !is_collapsed {
                for b in remote {
                    let display_name = strip_remote_prefix(&b.name);
                    entries.push(DisplayEntry {
                        label: format!("    {display_name}"),
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
                entries.push(DisplayEntry {
                    label: String::new(),
                    kind: EntryKind::Spacer,
                });
                let mut current_fork = String::new();
                for b in &forks {
                    if let CommitSource::Fork(ref owner) = b.source {
                        if *owner != current_fork {
                            current_fork = owner.clone();
                            let key = SectionKey::Fork(project_idx, owner.clone());
                            let fork_count = forks
                                .iter()
                                .filter(
                                    |fb| matches!(&fb.source, CommitSource::Fork(o) if o == owner),
                                )
                                .count();
                            let is_collapsed = collapsed.contains(&key);
                            let arrow = if is_collapsed { "\u{25b6}" } else { "\u{25bc}" };
                            entries.push(DisplayEntry {
                                label: format!("  {arrow} Fork: {owner}"),
                                kind: EntryKind::SectionHeader {
                                    key,
                                    count: fork_count,
                                },
                            });
                            if !is_collapsed {
                                for fb in &forks {
                                    if matches!(&fb.source, CommitSource::Fork(o) if o == owner) {
                                        entries.push(DisplayEntry {
                                            label: format!("    {}", fb.name),
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
            if !entries.is_empty()
                && !matches!(
                    entries.last().map(|e| &e.kind),
                    Some(EntryKind::Spacer | EntryKind::RepoHeader)
                )
            {
                entries.push(DisplayEntry {
                    label: String::new(),
                    kind: EntryKind::Spacer,
                });
            }
            let key = SectionKey::Tags(project_idx);
            let is_collapsed = collapsed.contains(&key);
            let arrow = if is_collapsed { "\u{25b6}" } else { "\u{25bc}" };
            let count = filtered_tags.len();
            entries.push(DisplayEntry {
                label: format!("  {arrow} Tags"),
                kind: EntryKind::SectionHeader { key, count },
            });
            if !is_collapsed {
                for t in filtered_tags {
                    entries.push(DisplayEntry {
                        label: format!("    ({})", t.name),
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

pub fn auto_collapse_defaults(projects: &[Project], filter: &str) -> HashSet<SectionKey> {
    let mut set = HashSet::new();
    for (project_idx, proj) in projects.iter().enumerate() {
        let remote_count = proj
            .repo_data
            .branches
            .iter()
            .filter(|b| {
                matches!(b.source, CommitSource::Local | CommitSource::Remote(_))
                    && b.name.contains('/')
            })
            .filter(|b| filter.is_empty() || b.name.contains(filter))
            .count();
        if remote_count > 15 {
            set.insert(SectionKey::Remote(project_idx));
        }
    }
    set
}

fn truncate_right(s: &str, max_width: usize) -> String {
    let w = UnicodeWidthStr::width(s);
    if w <= max_width {
        return s.to_string();
    }
    if max_width <= 1 {
        return "\u{2026}".to_string();
    }
    let target = max_width - 1;
    let mut total_w = 0;
    let mut end_byte = 0;
    for (byte_idx, ch) in s.char_indices() {
        let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if total_w + cw > target {
            break;
        }
        total_w += cw;
        end_byte = byte_idx + ch.len_utf8();
    }
    format!("{}\u{2026}", &s[..end_byte])
}

fn section_header_line(label: &str, count: usize, selected: bool, width: usize) -> Line<'static> {
    let bold_style = if selected {
        Style::default()
            .fg(theme::SECTION_HEADER_FG)
            .bg(theme::SELECTED_BG)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(theme::SECTION_HEADER_FG)
            .add_modifier(Modifier::BOLD)
    };

    let dim_style = if selected {
        Style::default().fg(theme::DIM_TEXT).bg(theme::SELECTED_BG)
    } else {
        Style::default().fg(theme::DIM_TEXT)
    };

    let sep_style = if selected {
        Style::default()
            .fg(theme::SECTION_SEPARATOR)
            .bg(theme::SELECTED_BG)
    } else {
        Style::default().fg(theme::SECTION_SEPARATOR)
    };

    let text_part = format!("{label} ({count})");
    let text_w = UnicodeWidthStr::width(text_part.as_str());
    let fill_len = width.saturating_sub(text_w + 1);
    let fill: String = "\u{2500}".repeat(fill_len);

    Line::from(vec![
        Span::styled(label.to_string(), bold_style),
        Span::styled(format!(" ({count}) "), dim_style),
        Span::styled(fill, sep_style),
    ])
}

fn split_branch_prefix(name: &str) -> Option<(&str, &str)> {
    let slash = name.find('/')?;
    let prefix = &name[..=slash];
    let rest = &name[slash + 1..];
    if rest.is_empty() {
        None
    } else {
        Some((prefix, rest))
    }
}

fn entry_line(
    entry: &DisplayEntry,
    selected: bool,
    max_width: usize,
    _focused: bool,
) -> Line<'static> {
    if matches!(entry.kind, EntryKind::Spacer) {
        return Line::from("");
    }

    if let EntryKind::SectionHeader { count, .. } = &entry.kind {
        return section_header_line(&entry.label, *count, selected, max_width);
    }

    // Two-tone branch names for branch entries
    let is_branch = matches!(
        entry.kind,
        EntryKind::LocalBranch { is_head: false, .. }
            | EntryKind::RemoteBranch { .. }
            | EntryKind::ForkBranch { .. }
    );

    if is_branch {
        let label = if max_width > 0 {
            truncate_right(&entry.label, max_width)
        } else {
            entry.label.clone()
        };

        let trimmed = label.trim_start();
        let indent = &label[..label.len() - trimmed.len()];

        let base_color = if matches!(entry.kind, EntryKind::ForkBranch { .. }) {
            theme::FORK_DIM
        } else {
            theme::branch_prefix_color(trimmed)
        };

        let bg = if selected {
            Some(theme::SELECTED_BG)
        } else {
            None
        };

        let mut indent_style = Style::default();
        if let Some(b) = bg {
            indent_style = indent_style.bg(b);
        }

        if let Some((prefix, rest)) = split_branch_prefix(trimmed) {
            let mut prefix_style = Style::default().fg(theme::DIM_PREFIX);
            let mut name_style = Style::default().fg(base_color);
            if matches!(entry.kind, EntryKind::ForkBranch { .. }) {
                prefix_style = prefix_style.add_modifier(Modifier::ITALIC);
                name_style = name_style.add_modifier(Modifier::ITALIC);
            }
            if let Some(b) = bg {
                prefix_style = prefix_style.bg(b);
                name_style = name_style.bg(b);
            }
            return Line::from(vec![
                Span::styled(indent.to_string(), indent_style),
                Span::styled(prefix.to_string(), prefix_style),
                Span::styled(rest.to_string(), name_style),
            ]);
        }

        let mut style = Style::default().fg(base_color);
        if matches!(entry.kind, EntryKind::ForkBranch { .. }) {
            style = style.add_modifier(Modifier::ITALIC);
        }
        if let Some(b) = bg {
            style = style.bg(b);
        }
        return Line::from(vec![
            Span::styled(indent.to_string(), indent_style),
            Span::styled(trimmed.to_string(), style),
        ]);
    }

    // HEAD branch, repo header, tags — single style
    let style = match &entry.kind {
        EntryKind::RepoHeader => Style::default()
            .fg(theme::ACTIVE_BORDER)
            .add_modifier(Modifier::BOLD),
        EntryKind::LocalBranch { is_head: true, .. } => Style::default()
            .fg(theme::HEAD_COLOR)
            .add_modifier(Modifier::BOLD),
        EntryKind::Tag { .. } => Style::default().fg(theme::TAG_COLOR),
        _ => Style::default(),
    };

    let style = if selected {
        style.bg(theme::SELECTED_BG)
    } else {
        style
    };

    let label = if !entry.is_header() && max_width > 0 {
        truncate_right(&entry.label, max_width)
    } else {
        entry.label.clone()
    };

    Line::from(Span::styled(label, style))
}
