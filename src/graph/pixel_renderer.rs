use crate::graph::types::{EdgeKind, RowLayout};
use crate::ui::theme::ThemePalette;
use ratatui::style::Color;
use tiny_skia::{
    FillRule, LineCap, LineJoin, Paint, PathBuilder, Pixmap, Stroke, Transform,
};

pub struct RenderParams {
    pub cell_width: u16,
    pub cell_height: u16,
    pub line_width: f32,
    pub commit_radius: f32,
    pub lane_width: f32,
}

impl RenderParams {
    pub fn from_cell_size(cell_width: u16, cell_height: u16) -> Self {
        let lane_width = cell_width as f32 * 2.0;
        Self {
            cell_width,
            cell_height,
            line_width: 2.0,
            commit_radius: (cell_width as f32 * 0.45).max(3.0),
            lane_width,
        }
    }

    pub fn cols_per_lane(&self) -> u16 {
        2
    }
}

fn color_to_skia(c: Color) -> tiny_skia::Color {
    match c {
        Color::Rgb(r, g, b) => tiny_skia::Color::from_rgba8(r, g, b, 255),
        Color::LightCyan => tiny_skia::Color::from_rgba8(0, 255, 255, 255),
        Color::LightYellow => tiny_skia::Color::from_rgba8(255, 255, 100, 255),
        Color::LightGreen => tiny_skia::Color::from_rgba8(100, 255, 100, 255),
        Color::LightRed => tiny_skia::Color::from_rgba8(255, 100, 100, 255),
        Color::Cyan => tiny_skia::Color::from_rgba8(0, 200, 200, 255),
        Color::Green => tiny_skia::Color::from_rgba8(0, 200, 0, 255),
        Color::Magenta => tiny_skia::Color::from_rgba8(200, 0, 200, 255),
        Color::Yellow => tiny_skia::Color::from_rgba8(200, 200, 0, 255),
        Color::Blue => tiny_skia::Color::from_rgba8(60, 60, 255, 255),
        Color::Red => tiny_skia::Color::from_rgba8(220, 60, 60, 255),
        Color::DarkGray => tiny_skia::Color::from_rgba8(100, 100, 100, 255),
        Color::White => tiny_skia::Color::from_rgba8(220, 220, 220, 255),
        _ => tiny_skia::Color::from_rgba8(180, 180, 180, 255),
    }
}

fn lane_center_x(lane: usize, lane_width: f32) -> f32 {
    lane as f32 * lane_width + lane_width / 2.0
}

pub fn render_row_image(
    layout: &RowLayout,
    params: &RenderParams,
    palette: &ThemePalette,
    trunk_count: usize,
) -> Option<Vec<u8>> {
    let num_lanes = layout
        .passthrough_lanes
        .iter()
        .map(|p| p.lane + 1)
        .chain(layout.edges.iter().map(|e| e.from_lane.max(e.to_lane) + 1))
        .max()
        .unwrap_or(0)
        .max(layout.commit_lane + 1);

    let img_width = (num_lanes as f32 * params.lane_width).ceil() as u32;
    let img_height = params.cell_height as u32;

    if img_width == 0 || img_height == 0 {
        return None;
    }

    let mut pixmap = Pixmap::new(img_width, img_height)?;

    let stroke = {
        let mut s = Stroke::default();
        s.width = params.line_width;
        s.line_cap = LineCap::Round;
        s.line_join = LineJoin::Round;
        s
    };

    // Passthrough lanes: vertical lines
    for pt in &layout.passthrough_lanes {
        let x = lane_center_x(pt.lane, params.lane_width);
        let color = palette.branch_color_by_identity(pt.color_index, trunk_count);
        let skia_color = color_to_skia(color);

        let mut paint = Paint::default();
        paint.set_color(skia_color);
        paint.anti_alias = true;

        if let Some(path) = {
            let mut pb = PathBuilder::new();
            pb.move_to(x, 0.0);
            pb.line_to(x, img_height as f32);
            pb.finish()
        } {
            pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }
    }

    // Edges
    for edge in &layout.edges {
        let color_index = match edge.kind {
            EdgeKind::MergeToParent { color_index } => color_index,
            EdgeKind::BranchToParent => edge.to_lane,
        };
        let color = palette.branch_color_by_identity(color_index, trunk_count);
        let skia_color = color_to_skia(color);

        let mut paint = Paint::default();
        paint.set_color(skia_color);
        paint.anti_alias = true;

        let from_x = lane_center_x(edge.from_lane, params.lane_width);
        let to_x = lane_center_x(edge.to_lane, params.lane_width);
        let h = img_height as f32;

        if let Some(path) = {
            let mut pb = PathBuilder::new();
            if edge.from_lane == edge.to_lane {
                pb.move_to(from_x, 0.0);
                pb.line_to(to_x, h);
            } else {
                pb.move_to(from_x, 0.0);
                pb.quad_to(from_x, h * 0.5, to_x, h);
            }
            pb.finish()
        } {
            pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }
    }

    // Commit node: filled circle
    let cx = lane_center_x(layout.commit_lane, params.lane_width);
    let cy = img_height as f32 / 2.0;
    let r = params.commit_radius;

    let commit_color = palette.branch_color_by_identity(layout.commit_color, trunk_count);
    let mut paint = Paint::default();
    paint.set_color(color_to_skia(commit_color));
    paint.anti_alias = true;

    if let Some(path) = {
        let mut pb = PathBuilder::new();
        pb.move_to(cx + r, cy);
        pb.quad_to(cx + r, cy + r * 0.55, cx + r * 0.55, cy + r);
        pb.quad_to(cx, cy + r, cx - r * 0.55, cy + r);
        pb.quad_to(cx - r, cy + r, cx - r, cy + r * 0.55);
        pb.quad_to(cx - r, cy, cx - r, cy - r * 0.55);
        pb.quad_to(cx - r, cy - r, cx - r * 0.55, cy - r);
        pb.quad_to(cx, cy - r, cx + r * 0.55, cy - r);
        pb.quad_to(cx + r, cy - r, cx + r, cy - r * 0.55);
        pb.quad_to(cx + r, cy, cx + r, cy);
        pb.close();
        pb.finish()
    } {
        pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
    }

    Some(pixmap.encode_png().ok()?)
}

pub fn num_lanes_for_layout(layout: &RowLayout) -> usize {
    layout
        .passthrough_lanes
        .iter()
        .map(|p| p.lane + 1)
        .chain(layout.edges.iter().map(|e| e.from_lane.max(e.to_lane) + 1))
        .max()
        .unwrap_or(0)
        .max(layout.commit_lane + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::types::{Edge, EdgeKind, LaneOccupant, RowLayout};
    use crate::ui::theme::palette_for_theme;

    fn make_simple_layout() -> RowLayout {
        RowLayout {
            commit_lane: 0,
            commit_color: 0,
            trunk_index: None,
            edges: vec![],
            passthrough_lanes: vec![],
            lane_branches: vec![Some(0)],
        }
    }

    #[test]
    fn render_produces_nonempty_png() {
        let layout = make_simple_layout();
        let params = RenderParams::from_cell_size(8, 16);
        let palette = palette_for_theme(None);
        let png = render_row_image(&layout, &params, &palette, 0);
        assert!(png.is_some());
        let bytes = png.unwrap();
        assert!(bytes.len() > 8);
        assert_eq!(&bytes[..4], &[0x89, 0x50, 0x4E, 0x47]); // PNG magic
    }

    #[test]
    fn render_with_edges_and_passthrough() {
        let layout = RowLayout {
            commit_lane: 0,
            commit_color: 1,
            trunk_index: None,
            edges: vec![Edge {
                from_lane: 0,
                to_lane: 1,
                kind: EdgeKind::BranchToParent,
            }],
            passthrough_lanes: vec![LaneOccupant {
                lane: 2,
                color_index: 2,
                trunk_index: None,
            }],
            lane_branches: vec![Some(0), Some(1), Some(2)],
        };
        let params = RenderParams::from_cell_size(8, 16);
        let palette = palette_for_theme(None);
        let png = render_row_image(&layout, &params, &palette, 0);
        assert!(png.is_some());
        let bytes = png.unwrap();
        assert!(bytes.len() > 100); // should have meaningful content
    }

    #[test]
    fn correct_image_dimensions() {
        let layout = make_simple_layout();
        let params = RenderParams::from_cell_size(10, 20);
        let palette = palette_for_theme(None);
        let png = render_row_image(&layout, &params, &palette, 0).unwrap();
        // Decode PNG header to check dimensions
        // PNG width is at bytes 16-19, height at 20-23 (big-endian u32)
        assert!(png.len() > 24);
        let width = u32::from_be_bytes([png[16], png[17], png[18], png[19]]);
        let height = u32::from_be_bytes([png[20], png[21], png[22], png[23]]);
        // 1 lane * 20px lane_width = 20 pixels wide, 20 pixels tall
        assert_eq!(width, 20);
        assert_eq!(height, 20);
    }
}
