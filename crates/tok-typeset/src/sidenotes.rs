//! Sidenotes & Marginalia layout engine with vertical collision avoidance (נושא 2.3).
//!
//! Places [`tok_core::Sidenote`] items in the outer or spine-side margin aligned to their
//! anchor line's vertical coordinate, pushing overlapping sidenotes downward while clamping
//! within the page's printable vertical bounds.

use crate::engine::TypesettingEngine;
use crate::geometry::{LineBox, PageLayoutBox, PhysicalRect, TextFrameBox};
use std::collections::HashMap;
use tok_core::id::{FractionalIndex, NodeId};
use tok_core::model::{DocumentRoot, ParagraphNode};
use tok_core::notes::{Sidenote, SidenoteConfig, SidenoteMarginSide};
use tok_core::styles::{ParagraphStyle, Progression, TextAlignment};

/// A sidenote candidate positioned at its desired anchor Y before collision resolution.
#[derive(Debug, Clone)]
struct SidenoteCandidate {
    sidenote: Sidenote,
    desired_top_y: f32,
    lines: Vec<LineBox>,
    total_height: f32,
    width_pt: f32,
    is_left_margin: bool,
}

/// Resolves vertical positions for a list of `(desired_top_y, height_pt)` blocks on one margin
/// so that no two blocks overlap and at least `vertical_gap_pt` separates consecutive blocks,
/// clamping within `[min_top_y, max_bottom_y]`.
///
/// Returns the resolved `top_y` for each item in the same order as input.
pub fn resolve_sidenote_positions(
    items: &[(f32, f32)],
    min_top_y: f32,
    max_bottom_y: f32,
    vertical_gap_pt: f32,
) -> Vec<f32> {
    if items.is_empty() {
        return Vec::new();
    }

    // Sort indices by desired_top_y stably
    let mut order: Vec<usize> = (0..items.len()).collect();
    order.sort_by(|&a, &b| {
        items[a]
            .0
            .partial_cmp(&items[b].0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.cmp(&b))
    });

    let mut placed_y = vec![0.0f32; items.len()];
    let mut cursor_y = min_top_y;

    // Forward pass: push overlapping notes downward
    for &idx in &order {
        let (desired_y, height) = items[idx];
        let y = desired_y.max(cursor_y);
        placed_y[idx] = y;
        cursor_y = y + height + vertical_gap_pt;
    }

    // Backward pass: if the bottom-most note overflows max_bottom_y, pull stack upward
    if let Some(&last_idx) = order.last() {
        let last_bottom = placed_y[last_idx] + items[last_idx].1;
        if last_bottom > max_bottom_y {
            let mut ceiling = max_bottom_y;
            for &idx in order.iter().rev() {
                let h = items[idx].1;
                let max_allowed_y = (ceiling - h).max(min_top_y);
                if placed_y[idx] > max_allowed_y {
                    placed_y[idx] = max_allowed_y;
                }
                ceiling = placed_y[idx] - vertical_gap_pt;
            }
        }
    }

    placed_y
}

fn resolve_margin_is_left(
    side: SidenoteMarginSide,
    page_index: usize,
    is_doc_rtl: bool,
) -> bool {
    let is_recto = page_index % 2 == 0; // page 1 (index 0) is Recto
    match side {
        SidenoteMarginSide::Left => true,
        SidenoteMarginSide::Right => false,
        SidenoteMarginSide::Outer => {
            if is_doc_rtl {
                // RTL book: Recto is left page (outer=left), Verso is right page (outer=right)
                is_recto
            } else {
                // LTR book: Recto is right page (outer=right), Verso is left page (outer=left)
                !is_recto
            }
        }
        SidenoteMarginSide::Inner => {
            if is_doc_rtl {
                !is_recto
            } else {
                is_recto
            }
        }
    }
}

/// Attaches all sidenotes defined on paragraphs in `doc` to their corresponding `PageLayoutBox`es,
/// computing anchor Y positions, breaking sidenote text to fit margin width, and resolving collisions.
pub fn attach_sidenotes_to_pages(
    engine: &TypesettingEngine,
    doc: &DocumentRoot,
    pages: &mut [PageLayoutBox],
) {
    // Collect paragraphs that have sidenotes along with their section's SidenoteConfig
    let mut para_sidenotes: HashMap<NodeId, (&[Sidenote], SidenoteConfig)> = HashMap::new();
    for sec in &doc.sections {
        let cfg = sec.sidenote_config.clone().unwrap_or_default();
        if !cfg.enabled {
            continue;
        }
        for flow in &sec.flows {
            for para in &flow.paragraphs {
                if !para.sidenotes.is_empty() {
                    para_sidenotes.insert(para.id, (para.sidenotes.as_slice(), cfg.clone()));
                }
            }
        }
    }

    if para_sidenotes.is_empty() {
        return;
    }

    let is_doc_rtl = doc.metadata.progression != Progression::Ltr;
    let margin_top = engine.config().margin_top_pt;
    let margin_bottom = engine.config().margin_bottom_pt;

    for page in pages.iter_mut() {
        if page.is_blank || page.frames.is_empty() {
            continue;
        }

        // Find body bounds on this page
        let body_left = page
            .frames
            .iter()
            .map(|f| f.rect.x)
            .fold(f32::INFINITY, f32::min);
        let body_right = page
            .frames
            .iter()
            .map(|f| f.rect.x + f.rect.width)
            .fold(f32::NEG_INFINITY, f32::max);

        // Find anchor lines for paragraphs present on this page
        let mut candidates: Vec<SidenoteCandidate> = Vec::new();
        let mut seen_sidenote_ids: std::collections::HashSet<NodeId> =
            std::collections::HashSet::new();

        for frame in &page.frames {
            for line in &frame.lines {
                let Some(pid) = line.paragraph_id else {
                    continue;
                };
                let Some(&(sidenotes, ref cfg)) = para_sidenotes.get(&pid) else {
                    continue;
                };

                for sn in sidenotes {
                    if !seen_sidenote_ids.insert(sn.id) {
                        continue;
                    }

                    let width_pt = sn.width_pt.unwrap_or(cfg.width_pt).max(24.0);
                    let font_family = sn
                        .font_family
                        .clone()
                        .unwrap_or_else(|| cfg.font_family.clone());
                    let font_size_pt = sn.font_size_pt.unwrap_or(cfg.font_size_pt);
                    let display_text = match &sn.marker {
                        Some(m) if !m.trim().is_empty() => format!("{} {}", m.trim(), sn.text),
                        _ => sn.text.clone(),
                    };

                    let mut sn_style = ParagraphStyle {
                        id: "sidenote-inline".to_string(),
                        name: "Sidenote".to_string(),
                        font_family,
                        font_size_pt,
                        line_height_pt: font_size_pt * cfg.line_height_ratio,
                        alignment: if is_doc_rtl {
                            TextAlignment::Right
                        } else {
                            TextAlignment::Left
                        },
                        ..Default::default()
                    };
                    if let Some(c) = &sn.color {
                        sn_style.color = c.clone();
                    }

                    let sn_para =
                        ParagraphNode::new(FractionalIndex::initial(), "sidenote-inline", &display_text);
                    let mut lines = engine.typeset_paragraph_shaped_with_style(
                        &sn_para,
                        Some(&sn_style),
                        &sn_style.font_family,
                        &crate::knuth_plass::MeasureProfile::uniform(width_pt),
                        sn_style.font_size_pt,
                        sn_style.font_size_pt * cfg.line_height_ratio,
                    );
                    if lines.is_empty() {
                        continue;
                    }

                    // Re-zero line baseline_y relative to the sidenote frame's top
                    let first_top = lines[0].baseline_y - lines[0].height;
                    let mut total_h = 0.0f32;
                    for (idx, l) in lines.iter_mut().enumerate() {
                        l.line_index = idx;
                        l.baseline_y -= first_top;
                        total_h = total_h.max(l.baseline_y);
                    }

                    let desired_top_y = (frame.rect.y + line.baseline_y - line.height)
                        .max(margin_top);
                    let side = sn.side.unwrap_or(cfg.default_side);
                    let is_left_margin = resolve_margin_is_left(side, page.page_index, is_doc_rtl);

                    candidates.push(SidenoteCandidate {
                        sidenote: sn.clone(),
                        desired_top_y,
                        lines,
                        total_height: total_h.max(font_size_pt),
                        width_pt,
                        is_left_margin,
                    });
                }
            }
        }

        if candidates.is_empty() {
            continue;
        }

        let max_bottom_y = (page.dimensions.height - margin_bottom).max(margin_top + 20.0);
        let gap_from_body = 10.0f32;
        let vertical_gap = 6.0f32;

        // Partition into left and right margin stacks for independent collision avoidance
        for is_left in [true, false] {
            let group_indices: Vec<usize> = candidates
                .iter()
                .enumerate()
                .filter(|(_, c)| c.is_left_margin == is_left)
                .map(|(i, _)| i)
                .collect();
            if group_indices.is_empty() {
                continue;
            }

            let items: Vec<(f32, f32)> = group_indices
                .iter()
                .map(|&i| (candidates[i].desired_top_y, candidates[i].total_height))
                .collect();
            let resolved_ys =
                resolve_sidenote_positions(&items, margin_top, max_bottom_y, vertical_gap);

            for (pos, &cand_idx) in group_indices.iter().enumerate() {
                let cand = &candidates[cand_idx];
                let y = resolved_ys[pos];
                let x = if is_left {
                    (body_left - gap_from_body - cand.width_pt).max(6.0)
                } else {
                    (body_right + gap_from_body)
                        .min((page.dimensions.width - cand.width_pt - 6.0).max(body_right))
                };

                page.frames.push(TextFrameBox {
                    frame_id: format!("sidenote-{}", cand.sidenote.id),
                    flow_id: "sidenotes".to_string(),
                    rect: PhysicalRect::new(x, y, cand.width_pt, cand.total_height),
                    lines: cand.lines.clone(),
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collision_avoidance_pushes_overlapping_sidenotes_downward() {
        let items = vec![(100.0, 30.0), (110.0, 25.0), (120.0, 20.0)];
        let ys = resolve_sidenote_positions(&items, 50.0, 700.0, 6.0);
        assert_eq!(ys[0], 100.0);
        assert_eq!(ys[1], 136.0); // 100 + 30 + 6
        assert_eq!(ys[2], 167.0); // 136 + 25 + 6
    }

    #[test]
    fn test_collision_avoidance_pulls_stack_up_when_hitting_bottom_margin() {
        let items = vec![(640.0, 30.0), (650.0, 30.0)];
        let ys = resolve_sidenote_positions(&items, 50.0, 680.0, 6.0);
        assert!(ys[1] + 30.0 <= 680.0 + 0.01);
        assert!(ys[0] + 30.0 + 6.0 <= ys[1] + 0.01);
    }
}
