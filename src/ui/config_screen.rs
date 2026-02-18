use crate::screen::{ConfigScreenState, ConfigSection, FieldMode};
use crate::ui::theme;
use ratatui::{
    buffer::Buffer as Buf,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Widget},
};

pub struct ConfigScreen<'a> {
    pub state: &'a ConfigScreenState,
}

impl<'a> Widget for ConfigScreen<'a> {
    fn render(self, area: Rect, buf: &mut Buf) {
        let bg_style = Style::default().bg(theme::APP_BG);
        for y in area.y..area.bottom() {
            for x in area.x..area.right() {
                buf[(x, y)].set_style(bg_style);
            }
        }

        let outer = Block::default()
            .title(" arachne config ")
            .title_style(
                Style::default()
                    .fg(theme::ACCENT)
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::ACTIVE_PANEL_BORDER));
        let inner = outer.inner(area);
        outer.render(area, buf);

        if inner.height < 4 || inner.width < 20 {
            return;
        }

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // tabs
                Constraint::Length(1), // separator
                Constraint::Min(1),   // content
                Constraint::Length(1), // footer
            ])
            .split(inner);

        let tabs_area = layout[0];
        let content_area = layout[2];
        let footer_area = layout[3];

        render_tabs(buf, tabs_area, self.state.active_section);
        render_section_content(buf, content_area, self.state);
        render_footer(buf, footer_area, self.state);
    }
}

fn render_tabs(buf: &mut Buf, area: Rect, active: ConfigSection) {
    let mut x = area.x + 1;
    for section in ConfigSection::ALL {
        let is_active = *section == active;
        let style = if is_active {
            Style::default()
                .fg(theme::ACCENT)
                .bg(theme::SELECTED_BG)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::DIM_TEXT).bg(theme::APP_BG)
        };
        let label = format!(" {} ", section.label());
        let w = label.len() as u16;
        if x + w > area.right() {
            break;
        }
        buf.set_line(x, area.y, &Line::from(Span::styled(label, style)), w);
        x += w + 1;
    }
}

fn render_section_content(buf: &mut Buf, area: Rect, state: &ConfigScreenState) {
    if area.height == 0 {
        return;
    }

    match state.active_section {
        ConfigSection::Repos => render_repos_section(buf, area, state),
        ConfigSection::Trunk => render_trunk_section(buf, area, state),
        ConfigSection::Theme => render_theme_section(buf, area, state),
        ConfigSection::Profiles => render_profiles_section(buf, area, state),
    }
}

fn render_repos_section(buf: &mut Buf, area: Rect, state: &ConfigScreenState) {
    let repos = state.draft.resolved_repos();
    let x = area.x + 2;
    let max_w = area.width.saturating_sub(4) as u16;

    for (i, entry) in repos.iter().enumerate() {
        let y = area.y + i as u16;
        if y >= area.bottom() {
            break;
        }

        let is_selected = i == state.cursor;
        let is_editing =
            is_selected && matches!(state.field_mode, FieldMode::Editing(_));

        let path_str = entry.path.to_string_lossy();
        let name_str = entry
            .name
            .as_deref()
            .unwrap_or("(auto-detect)");

        let line = if is_editing {
            if let FieldMode::Editing(ref text) = state.field_mode {
                Line::from(vec![
                    Span::styled(
                        format!("{text}\u{258c}"),
                        Style::default().fg(theme::FILTER_COLOR).bg(theme::SELECTED_BG),
                    ),
                    Span::styled(
                        format!("  {name_str}"),
                        Style::default().fg(theme::DIM_TEXT).bg(theme::SELECTED_BG),
                    ),
                ])
            } else {
                Line::raw("")
            }
        } else {
            let bg = if is_selected {
                theme::SELECTED_BG
            } else {
                theme::APP_BG
            };
            Line::from(vec![
                Span::styled(
                    path_str.to_string(),
                    Style::default().fg(theme::ACCENT).bg(bg),
                ),
                Span::styled(
                    format!("  {name_str}"),
                    Style::default().fg(theme::DIM_TEXT).bg(bg),
                ),
            ])
        };

        buf.set_line(x, y, &line, max_w);
        if is_selected {
            let sel_style = Style::default().bg(theme::SELECTED_BG);
            for cx in area.x..area.right() {
                buf[(cx, y)].set_style(sel_style);
            }
        }
    }

    let hint_y = area.y + repos.len() as u16 + 1;
    if hint_y < area.bottom() {
        let hint = Line::from(Span::styled(
            "  Enter: edit path  a: add  x: remove",
            Style::default().fg(theme::DIM_TEXT),
        ));
        buf.set_line(area.x, hint_y, &hint, area.width);
    }
}

fn render_trunk_section(buf: &mut Buf, area: Rect, state: &ConfigScreenState) {
    let x = area.x + 2;
    let max_w = area.width.saturating_sub(4) as u16;

    for (i, branch) in state.draft.trunk_branches.iter().enumerate() {
        let y = area.y + i as u16;
        if y >= area.bottom() {
            break;
        }

        let is_selected = i == state.cursor;
        let is_editing =
            is_selected && matches!(state.field_mode, FieldMode::Editing(_));

        let line = if is_editing {
            if let FieldMode::Editing(ref text) = state.field_mode {
                Line::from(Span::styled(
                    format!("{text}\u{258c}"),
                    Style::default().fg(theme::FILTER_COLOR).bg(theme::SELECTED_BG),
                ))
            } else {
                Line::raw("")
            }
        } else {
            let bg = if is_selected {
                theme::SELECTED_BG
            } else {
                theme::APP_BG
            };
            Line::from(Span::styled(
                branch.clone(),
                Style::default().bg(bg),
            ))
        };

        buf.set_line(x, y, &line, max_w);
        if is_selected {
            let sel_style = Style::default().bg(theme::SELECTED_BG);
            for cx in area.x..area.right() {
                buf[(cx, y)].set_style(sel_style);
            }
        }
    }

    let hint_y = area.y + state.draft.trunk_branches.len() as u16 + 1;
    if hint_y < area.bottom() {
        let hint = Line::from(Span::styled(
            "  Enter: edit  a: add  x: remove",
            Style::default().fg(theme::DIM_TEXT),
        ));
        buf.set_line(area.x, hint_y, &hint, area.width);
    }
}

fn render_theme_section(buf: &mut Buf, area: Rect, state: &ConfigScreenState) {
    let presets = [
        "Arachne Purple",
        "Midnight",
        "Emerald",
        "Amber",
        "Frost",
    ];
    let x = area.x + 2;
    let max_w = area.width.saturating_sub(4) as u16;

    let active_theme = state.draft.theme.as_deref().unwrap_or("Arachne Purple");

    for (i, name) in presets.iter().enumerate() {
        let y = area.y + i as u16;
        if y >= area.bottom() {
            break;
        }

        let is_selected = i == state.cursor;
        let is_active = *name == active_theme;
        let marker = if is_active { "\u{25b8} " } else { "  " };

        let bg = if is_selected {
            theme::SELECTED_BG
        } else {
            theme::APP_BG
        };

        let line = Line::from(vec![
            Span::styled(
                marker.to_string(),
                Style::default().fg(theme::ACCENT).bg(bg),
            ),
            Span::styled(
                name.to_string(),
                Style::default().bg(bg),
            ),
        ]);

        buf.set_line(x, y, &line, max_w);
        if is_selected {
            let sel = Style::default().bg(bg);
            for cx in area.x..area.right() {
                buf[(cx, y)].set_style(sel);
            }
        }
    }

    let hint_y = area.y + presets.len() as u16 + 1;
    if hint_y < area.bottom() {
        let hint = Line::from(Span::styled(
            "  Enter/Space: select theme",
            Style::default().fg(theme::DIM_TEXT),
        ));
        buf.set_line(area.x, hint_y, &hint, area.width);
    }
}

fn render_profiles_section(buf: &mut Buf, area: Rect, state: &ConfigScreenState) {
    let x = area.x + 2;
    let max_w = area.width.saturating_sub(4) as u16;

    if state.draft.profiles.is_empty() {
        let y = area.y;
        if y < area.bottom() {
            let line = Line::from(Span::styled(
                "No profiles configured. Press 'a' to create one.",
                Style::default().fg(theme::DIM_TEXT),
            ));
            buf.set_line(x, y, &line, max_w);
        }
        return;
    }

    let active_profile = state.draft.active_profile.as_deref().unwrap_or("");

    for (i, profile) in state.draft.profiles.iter().enumerate() {
        let y = area.y + i as u16;
        if y >= area.bottom() {
            break;
        }

        let is_selected = i == state.cursor;
        let is_active = profile.name == active_profile;
        let marker = if is_active { "\u{25b8} " } else { "  " };

        let bg = if is_selected {
            theme::SELECTED_BG
        } else {
            theme::APP_BG
        };

        let token_hint = if profile.github_token.is_some() {
            " (token set)"
        } else {
            " (no token)"
        };

        let line = Line::from(vec![
            Span::styled(marker.to_string(), Style::default().fg(theme::ACCENT).bg(bg)),
            Span::styled(profile.name.clone(), Style::default().bg(bg)),
            Span::styled(
                token_hint.to_string(),
                Style::default().fg(theme::DIM_TEXT).bg(bg),
            ),
        ]);

        buf.set_line(x, y, &line, max_w);
        if is_selected {
            let sel = Style::default().bg(bg);
            for cx in area.x..area.right() {
                buf[(cx, y)].set_style(sel);
            }
        }
    }

    let hint_y = area.y + state.draft.profiles.len() as u16 + 1;
    if hint_y < area.bottom() {
        let hint = Line::from(Span::styled(
            "  Enter: activate  a: add  x: remove",
            Style::default().fg(theme::DIM_TEXT),
        ));
        buf.set_line(area.x, hint_y, &hint, area.width);
    }
}

fn render_footer(buf: &mut Buf, area: Rect, state: &ConfigScreenState) {
    let dirty_marker = if state.dirty { " [modified]" } else { "" };

    let line = Line::from(vec![
        Span::styled(
            " Ctrl-S: save",
            Style::default()
                .fg(theme::ACCENT)
                .bg(theme::APP_BG)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "  Esc: back",
            Style::default().fg(theme::DIM_TEXT).bg(theme::APP_BG),
        ),
        Span::styled(
            "  Tab: section",
            Style::default().fg(theme::DIM_TEXT).bg(theme::APP_BG),
        ),
        Span::styled(
            dirty_marker.to_string(),
            Style::default().fg(theme::WARN_FG).bg(theme::APP_BG),
        ),
    ]);
    buf.set_line(area.x, area.y, &line, area.width);
}
