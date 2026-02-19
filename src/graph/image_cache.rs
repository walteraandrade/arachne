use std::collections::HashMap;

use crate::graph::pixel_renderer::{num_lanes_for_layout, render_row_image, RenderParams};
use crate::graph::types::RowLayout;
use crate::kitty_protocol::encode_kitty_image;
use crate::ui::theme::ThemePalette;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct CacheKey {
    commit_lane: usize,
    commit_color: usize,
    edges: Vec<(usize, usize, usize)>, // (from, to, color_index)
    passthrough: Vec<(usize, usize)>,   // (lane, color_index)
}

impl CacheKey {
    fn from_layout(layout: &RowLayout) -> Self {
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
        }
    }
}

pub struct ImageCache {
    png_cache: HashMap<CacheKey, Vec<u8>>,
    encoded_cache: HashMap<CacheKey, String>,
    next_image_id: u32,
}

impl ImageCache {
    pub fn new() -> Self {
        Self {
            png_cache: HashMap::new(),
            encoded_cache: HashMap::new(),
            next_image_id: 1,
        }
    }

    pub fn clear(&mut self) {
        self.png_cache.clear();
        self.encoded_cache.clear();
    }

    pub fn get_encoded(
        &mut self,
        layout: &RowLayout,
        params: &RenderParams,
        palette: &ThemePalette,
        trunk_count: usize,
    ) -> Option<&str> {
        let key = CacheKey::from_layout(layout);

        if !self.encoded_cache.contains_key(&key) {
            let png = if let Some(cached_png) = self.png_cache.get(&key) {
                cached_png.clone()
            } else {
                let png = render_row_image(layout, params, palette, trunk_count)?;
                self.png_cache.insert(key.clone(), png.clone());
                png
            };

            let num_lanes = num_lanes_for_layout(layout);
            let cols = (num_lanes as u16) * params.cols_per_lane();
            let image_id = self.next_image_id;
            self.next_image_id = self.next_image_id.wrapping_add(1);

            let encoded = encode_kitty_image(image_id, &png, cols, 1);
            self.encoded_cache.insert(key.clone(), encoded);
        }

        self.encoded_cache.get(&key).map(|s| s.as_str())
    }

    #[allow(dead_code)]
    pub fn stats(&self) -> (usize, usize) {
        (self.png_cache.len(), self.encoded_cache.len())
    }
}
