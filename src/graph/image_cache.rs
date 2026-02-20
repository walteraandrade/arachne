use std::collections::HashMap;

use crate::graph::pixel_renderer::{render_row_image, RenderParams};
use crate::graph::types::RowLayout;
use crate::ui::theme::ThemePalette;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct CacheKey {
    commit_lane: usize,
    commit_color: usize,
    edges: Vec<(usize, usize, usize)>, // (from, to, color_index)
    passthrough: Vec<(usize, usize)>,  // (lane, color_index)
    trunk_count: usize,
}

impl CacheKey {
    fn from_layout(layout: &RowLayout, trunk_count: usize) -> Self {
        let edges = layout
            .edges
            .iter()
            .map(|e| {
                let ci = match e.kind {
                    crate::graph::types::EdgeKind::MergeToParent { color_index } => color_index,
                    crate::graph::types::EdgeKind::BranchToParent => e.to_lane,
                };
                (e.from_lane, e.to_lane, ci)
            })
            .collect();
        let passthrough = layout
            .passthrough_lanes
            .iter()
            .map(|p| (p.lane, p.color_index))
            .collect();
        CacheKey {
            commit_lane: layout.commit_lane,
            commit_color: layout.commit_color,
            edges,
            passthrough,
            trunk_count,
        }
    }
}

const MAX_CACHE_ENTRIES: usize = 4096;

pub struct ImageCache {
    png_cache: HashMap<CacheKey, Vec<u8>>,
    max_lanes: usize,
    dirty: bool,
}

impl ImageCache {
    pub fn new() -> Self {
        Self {
            png_cache: HashMap::new(),
            max_lanes: 0,
            dirty: false,
        }
    }

    pub fn take_dirty(&mut self) -> bool {
        let was = self.dirty;
        self.dirty = false;
        was
    }

    pub fn clear(&mut self, max_lanes: usize) {
        self.png_cache.clear();
        self.max_lanes = max_lanes;
        self.dirty = true;
    }

    pub fn get_png(
        &mut self,
        layout: &RowLayout,
        params: &RenderParams,
        palette: &ThemePalette,
        trunk_count: usize,
    ) -> Option<&[u8]> {
        let key = CacheKey::from_layout(layout, trunk_count);

        if !self.png_cache.contains_key(&key) {
            if self.png_cache.len() >= MAX_CACHE_ENTRIES {
                self.png_cache.clear();
            }
            let png = render_row_image(layout, params, palette, trunk_count, self.max_lanes)?;
            self.png_cache.insert(key.clone(), png);
        }

        self.png_cache.get(&key).map(|v| v.as_slice())
    }
}

impl Default for ImageCache {
    fn default() -> Self {
        Self::new()
    }
}
