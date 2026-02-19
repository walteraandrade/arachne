use ratatui::style::Color;

pub const THEME_NAMES: &[&str] = &[
    "Arachne Purple",
    "Midnight",
    "Emerald",
    "Amber",
    "Frost",
];

#[derive(Clone)]
pub struct ThemePalette {
    pub app_bg: Color,
    pub header_bg: Color,
    pub status_bg: Color,
    pub content_bg: Color,
    pub content_fg: Color,
    pub selected_bg: Color,
    pub unfocused_sel_bg: Color,
    pub lane_header_bg: Color,

    pub accent: Color,
    pub selected_accent: Color,
    pub dim_text: Color,
    pub dim_prefix: Color,
    pub panel_label: Color,
    pub section_header_fg: Color,

    pub separator: Color,
    pub section_separator: Color,

    pub active_panel_border: Color,
    pub inactive_panel_border: Color,
    pub active_border: Color,

    pub filter_color: Color,
    pub head_color: Color,
    pub tag_color: Color,
    pub fork_dim: Color,
    pub error_fg: Color,
    pub warn_fg: Color,

    pub branch_colors: &'static [Color],
    pub trunk_colors: &'static [Color],
}

impl ThemePalette {
    pub fn branch_color_by_identity(&self, branch_index: usize, trunk_count: usize) -> Color {
        if branch_index < trunk_count && branch_index < self.trunk_colors.len() {
            self.trunk_colors[branch_index]
        } else {
            self.branch_colors
                [branch_index.saturating_sub(trunk_count) % self.branch_colors.len()]
        }
    }

    pub fn with_remote_tint(&self) -> ThemePalette {
        let mut p = self.clone();
        p.content_bg = dim_color(p.content_bg, 4);
        p.content_fg = dim_color(p.content_fg, 50);
        p.selected_bg = dim_color(p.selected_bg, 10);
        p.status_bg = dim_color(p.status_bg, 8);
        p
    }
}

fn dim_color(c: Color, amount: u8) -> Color {
    match c {
        Color::Rgb(r, g, b) => Color::Rgb(
            r.saturating_sub(amount),
            g.saturating_sub(amount),
            b.saturating_sub(amount),
        ),
        other => other,
    }
}

pub fn palette_for_theme(name: Option<&str>) -> ThemePalette {
    match name.unwrap_or("Arachne Purple") {
        "Midnight" => midnight(),
        "Emerald" => emerald(),
        "Amber" => amber(),
        "Frost" => frost(),
        _ => arachne_purple(),
    }
}

pub fn branch_prefix_color(name: &str) -> Color {
    if name.starts_with("feat/") || name.starts_with("feature/") {
        Color::Cyan
    } else if name.starts_with("fix/") || name.starts_with("bugfix/") || name.starts_with("hotfix/")
    {
        Color::LightRed
    } else if name.starts_with("chore/") {
        Color::Rgb(140, 140, 160)
    } else if name.starts_with("release/") {
        Color::Yellow
    } else if name.starts_with("cherry-pick/") {
        Color::Magenta
    } else {
        Color::White
    }
}

// ── Arachne Purple (default) ──────────────────────────────────────

static PURPLE_BRANCHES: &[Color] = &[
    Color::Green,
    Color::Cyan,
    Color::Magenta,
    Color::Yellow,
    Color::Blue,
    Color::Red,
];

static PURPLE_TRUNKS: &[Color] = &[Color::LightCyan, Color::LightYellow, Color::LightGreen];

fn arachne_purple() -> ThemePalette {
    ThemePalette {
        app_bg: Color::Rgb(22, 22, 34),
        header_bg: Color::Rgb(25, 25, 38),
        status_bg: Color::Rgb(30, 30, 40),
        content_bg: Color::Rgb(24, 24, 36),
        content_fg: Color::Rgb(220, 220, 230),
        selected_bg: Color::Rgb(62, 45, 100),
        unfocused_sel_bg: Color::Rgb(42, 35, 65),
        lane_header_bg: Color::Rgb(28, 28, 42),

        accent: Color::Rgb(140, 115, 200),
        selected_accent: Color::Rgb(160, 130, 220),
        dim_text: Color::Rgb(100, 100, 120),
        dim_prefix: Color::Rgb(90, 90, 110),
        panel_label: Color::Rgb(100, 95, 130),
        section_header_fg: Color::White,

        separator: Color::Rgb(55, 55, 75),
        section_separator: Color::Rgb(60, 60, 80),

        active_panel_border: Color::Rgb(140, 115, 200),
        inactive_panel_border: Color::Rgb(55, 55, 75),
        active_border: Color::Rgb(120, 120, 180),

        filter_color: Color::Cyan,
        head_color: Color::Green,
        tag_color: Color::Yellow,
        fork_dim: Color::DarkGray,
        error_fg: Color::LightRed,
        warn_fg: Color::Yellow,

        branch_colors: PURPLE_BRANCHES,
        trunk_colors: PURPLE_TRUNKS,
    }
}

// ── Midnight ──────────────────────────────────────────────────────

static MIDNIGHT_BRANCHES: &[Color] = &[
    Color::Rgb(80, 200, 160),
    Color::Rgb(100, 180, 255),
    Color::Rgb(180, 130, 220),
    Color::Rgb(200, 180, 100),
    Color::Rgb(80, 140, 220),
    Color::Rgb(220, 120, 120),
];

static MIDNIGHT_TRUNKS: &[Color] = &[
    Color::Rgb(140, 220, 255),
    Color::Rgb(200, 220, 140),
    Color::Rgb(140, 240, 200),
];

fn midnight() -> ThemePalette {
    ThemePalette {
        app_bg: Color::Rgb(12, 16, 28),
        header_bg: Color::Rgb(15, 20, 35),
        status_bg: Color::Rgb(18, 22, 38),
        content_bg: Color::Rgb(14, 18, 32),
        content_fg: Color::Rgb(190, 200, 220),
        selected_bg: Color::Rgb(25, 45, 80),
        unfocused_sel_bg: Color::Rgb(20, 35, 60),
        lane_header_bg: Color::Rgb(16, 22, 38),

        accent: Color::Rgb(100, 150, 220),
        selected_accent: Color::Rgb(120, 170, 240),
        dim_text: Color::Rgb(80, 95, 120),
        dim_prefix: Color::Rgb(70, 85, 110),
        panel_label: Color::Rgb(80, 100, 140),
        section_header_fg: Color::White,

        separator: Color::Rgb(40, 50, 70),
        section_separator: Color::Rgb(45, 55, 75),

        active_panel_border: Color::Rgb(100, 150, 220),
        inactive_panel_border: Color::Rgb(40, 50, 70),
        active_border: Color::Rgb(90, 130, 200),

        filter_color: Color::Rgb(100, 200, 255),
        head_color: Color::Rgb(80, 200, 160),
        tag_color: Color::Rgb(200, 180, 100),
        fork_dim: Color::DarkGray,
        error_fg: Color::LightRed,
        warn_fg: Color::Yellow,

        branch_colors: MIDNIGHT_BRANCHES,
        trunk_colors: MIDNIGHT_TRUNKS,
    }
}

// ── Emerald ───────────────────────────────────────────────────────

static EMERALD_BRANCHES: &[Color] = &[
    Color::Rgb(100, 220, 140),
    Color::Rgb(80, 200, 200),
    Color::Rgb(180, 140, 200),
    Color::Rgb(200, 200, 100),
    Color::Rgb(100, 160, 200),
    Color::Rgb(220, 130, 110),
];

static EMERALD_TRUNKS: &[Color] = &[
    Color::Rgb(140, 240, 200),
    Color::Rgb(200, 230, 140),
    Color::Rgb(100, 220, 220),
];

fn emerald() -> ThemePalette {
    ThemePalette {
        app_bg: Color::Rgb(14, 24, 18),
        header_bg: Color::Rgb(18, 28, 22),
        status_bg: Color::Rgb(20, 32, 24),
        content_bg: Color::Rgb(16, 26, 20),
        content_fg: Color::Rgb(200, 220, 200),
        selected_bg: Color::Rgb(30, 65, 45),
        unfocused_sel_bg: Color::Rgb(25, 50, 35),
        lane_header_bg: Color::Rgb(18, 30, 24),

        accent: Color::Rgb(80, 200, 120),
        selected_accent: Color::Rgb(100, 220, 140),
        dim_text: Color::Rgb(80, 110, 90),
        dim_prefix: Color::Rgb(70, 100, 80),
        panel_label: Color::Rgb(80, 120, 95),
        section_header_fg: Color::White,

        separator: Color::Rgb(40, 60, 48),
        section_separator: Color::Rgb(45, 65, 52),

        active_panel_border: Color::Rgb(80, 200, 120),
        inactive_panel_border: Color::Rgb(40, 60, 48),
        active_border: Color::Rgb(70, 160, 100),

        filter_color: Color::Rgb(120, 230, 160),
        head_color: Color::Rgb(100, 230, 150),
        tag_color: Color::Rgb(200, 200, 100),
        fork_dim: Color::DarkGray,
        error_fg: Color::LightRed,
        warn_fg: Color::Yellow,

        branch_colors: EMERALD_BRANCHES,
        trunk_colors: EMERALD_TRUNKS,
    }
}

// ── Amber ─────────────────────────────────────────────────────────

static AMBER_BRANCHES: &[Color] = &[
    Color::Rgb(140, 200, 80),
    Color::Rgb(100, 190, 180),
    Color::Rgb(200, 140, 180),
    Color::Rgb(220, 190, 80),
    Color::Rgb(120, 160, 200),
    Color::Rgb(220, 120, 100),
];

static AMBER_TRUNKS: &[Color] = &[
    Color::Rgb(200, 230, 140),
    Color::Rgb(240, 210, 140),
    Color::Rgb(160, 220, 180),
];

fn amber() -> ThemePalette {
    ThemePalette {
        app_bg: Color::Rgb(26, 20, 14),
        header_bg: Color::Rgb(30, 24, 18),
        status_bg: Color::Rgb(34, 28, 22),
        content_bg: Color::Rgb(28, 22, 16),
        content_fg: Color::Rgb(220, 210, 190),
        selected_bg: Color::Rgb(80, 55, 25),
        unfocused_sel_bg: Color::Rgb(60, 45, 22),
        lane_header_bg: Color::Rgb(32, 26, 20),

        accent: Color::Rgb(220, 170, 60),
        selected_accent: Color::Rgb(240, 190, 80),
        dim_text: Color::Rgb(120, 105, 80),
        dim_prefix: Color::Rgb(110, 95, 70),
        panel_label: Color::Rgb(140, 120, 80),
        section_header_fg: Color::White,

        separator: Color::Rgb(65, 55, 40),
        section_separator: Color::Rgb(70, 60, 45),

        active_panel_border: Color::Rgb(220, 170, 60),
        inactive_panel_border: Color::Rgb(65, 55, 40),
        active_border: Color::Rgb(180, 140, 50),

        filter_color: Color::Rgb(240, 200, 100),
        head_color: Color::Rgb(140, 200, 80),
        tag_color: Color::Rgb(220, 190, 80),
        fork_dim: Color::DarkGray,
        error_fg: Color::LightRed,
        warn_fg: Color::Rgb(240, 200, 80),

        branch_colors: AMBER_BRANCHES,
        trunk_colors: AMBER_TRUNKS,
    }
}

// ── Frost ─────────────────────────────────────────────────────────

static FROST_BRANCHES: &[Color] = &[
    Color::Rgb(100, 210, 160),
    Color::Rgb(120, 190, 230),
    Color::Rgb(190, 150, 210),
    Color::Rgb(210, 200, 120),
    Color::Rgb(100, 150, 210),
    Color::Rgb(210, 130, 130),
];

static FROST_TRUNKS: &[Color] = &[
    Color::Rgb(160, 220, 240),
    Color::Rgb(200, 220, 160),
    Color::Rgb(160, 230, 200),
];

fn frost() -> ThemePalette {
    ThemePalette {
        app_bg: Color::Rgb(28, 32, 38),
        header_bg: Color::Rgb(32, 36, 42),
        status_bg: Color::Rgb(36, 40, 46),
        content_bg: Color::Rgb(30, 34, 40),
        content_fg: Color::Rgb(210, 220, 230),
        selected_bg: Color::Rgb(50, 65, 85),
        unfocused_sel_bg: Color::Rgb(42, 52, 68),
        lane_header_bg: Color::Rgb(34, 38, 44),

        accent: Color::Rgb(140, 190, 230),
        selected_accent: Color::Rgb(160, 210, 250),
        dim_text: Color::Rgb(100, 115, 130),
        dim_prefix: Color::Rgb(90, 105, 120),
        panel_label: Color::Rgb(110, 130, 155),
        section_header_fg: Color::White,

        separator: Color::Rgb(55, 65, 78),
        section_separator: Color::Rgb(60, 70, 82),

        active_panel_border: Color::Rgb(140, 190, 230),
        inactive_panel_border: Color::Rgb(55, 65, 78),
        active_border: Color::Rgb(120, 170, 210),

        filter_color: Color::Rgb(140, 210, 250),
        head_color: Color::Rgb(100, 210, 160),
        tag_color: Color::Rgb(210, 200, 120),
        fork_dim: Color::DarkGray,
        error_fg: Color::LightRed,
        warn_fg: Color::Yellow,

        branch_colors: FROST_BRANCHES,
        trunk_colors: FROST_TRUNKS,
    }
}
