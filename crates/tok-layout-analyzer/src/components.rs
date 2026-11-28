//! Connected-component analysis for raster line-height and glyph-density estimation.

use crate::geometry::NormalizedBBox;
use crate::raster::GrayImage;

/// Summary statistics of ink connected components within a region.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ComponentStats {
    pub count: usize,
    pub median_height_norm: f32,
    pub median_width_norm: f32,
    pub median_aspect_ratio: f32,
    pub ink_density: f32,
}

/// Estimates line height and glyph aspect ratio inside a normalized region using 4-connected
/// component labeling (bounded to a fast scanline union-find).
pub fn analyze_region_components(
    img: &GrayImage,
    region: NormalizedBBox,
    ink_threshold: u8,
) -> ComponentStats {
    let w = img.width as usize;
    let h = img.height as usize;
    if w == 0 || h == 0 {
        return ComponentStats::default();
    }

    let x0 = ((region.x0 * w as f32).floor() as usize).min(w);
    let y0 = ((region.y0 * h as f32).floor() as usize).min(h);
    let x1 = ((region.x1 * w as f32).ceil() as usize).min(w);
    let y1 = ((region.y1 * h as f32).ceil() as usize).min(h);

    if x1 <= x0 || y1 <= y0 {
        return ComponentStats::default();
    }

    let rw = x1 - x0;
    let rh = y1 - y0;
    let mut visited = vec![false; rw * rh];
    let mut heights = Vec::new();
    let mut widths = Vec::new();
    let mut ratios = Vec::new();
    let mut ink_pixels = 0usize;
    let mut queue = Vec::with_capacity(64);

    for ly in 0..rh {
        for lx in 0..rw {
            let idx = ly * rw + lx;
            if visited[idx] {
                continue;
            }
            let px = img.data[(y0 + ly) * w + (x0 + lx)];
            if px >= ink_threshold {
                continue;
            }

            visited[idx] = true;
            queue.clear();
            queue.push((lx, ly));
            let mut min_x = lx;
            let mut max_x = lx;
            let mut min_y = ly;
            let mut max_y = ly;
            let mut comp_area = 0usize;

            let mut head = 0usize;
            while head < queue.len() {
                let (cx, cy) = queue[head];
                head += 1;
                comp_area += 1;
                ink_pixels += 1;
                min_x = min_x.min(cx);
                max_x = max_x.max(cx);
                min_y = min_y.min(cy);
                max_y = max_y.max(cy);

                let neighbors = [
                    (cx.wrapping_sub(1), cy),
                    (cx + 1, cy),
                    (cx, cy.wrapping_sub(1)),
                    (cx, cy + 1),
                ];
                for (nx, ny) in neighbors {
                    if nx < rw && ny < rh {
                        let nidx = ny * rw + nx;
                        if !visited[nidx] && img.data[(y0 + ny) * w + (x0 + nx)] < ink_threshold {
                            visited[nidx] = true;
                            queue.push((nx, ny));
                        }
                    }
                }
            }

            let cw = (max_x - min_x + 1) as f32;
            let ch = (max_y - min_y + 1) as f32;
            if comp_area >= 3 && ch >= 3.0 && ch <= (h as f32 * 0.15) {
                heights.push(ch / h as f32);
                widths.push(cw / w as f32);
                ratios.push(cw / ch);
            }
        }
    }

    let ink_density = ink_pixels as f32 / (rw * rh) as f32;
    if heights.is_empty() {
        return ComponentStats {
            count: 0,
            ink_density,
            ..Default::default()
        };
    }

    heights.sort_by(|a, b| a.total_cmp(b));
    widths.sort_by(|a, b| a.total_cmp(b));
    ratios.sort_by(|a, b| a.total_cmp(b));
    let mid = heights.len() / 2;

    ComponentStats {
        count: heights.len(),
        median_height_norm: heights[mid],
        median_width_norm: widths[mid],
        median_aspect_ratio: ratios[mid],
        ink_density,
    }
}
