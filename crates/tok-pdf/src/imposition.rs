//! Booklet Imposition, Signatures & Creep Compensation (נושא 3.2).
//!
//! Generates 2-up saddle-stitch / multi-signature imposition plans for duplex printing
//! (e.g., 16-page signatures: [16, 1], [2, 15], ...) with RTL Hebrew binding support
//! and geometric paper-thickness creep compensation.

use serde::{Deserialize, Serialize};
use tok_typeset::{PageLayoutBox, PhysicalRect};

/// Configuration for booklet / signature imposition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImpositionConfig {
    /// Enable 2-up signature imposition.
    pub enabled: bool,
    /// Pages per signature (must be a multiple of 4, e.g., 16 or 8).
    /// `0` means a single saddle-stitch signature covering all pages (rounded up to multiple of 4).
    pub signature_size: usize,
    /// Paper caliper / thickness per sheet in points (e.g., 0.25 pt ~ 0.088 mm for 80gsm offset).
    pub paper_thickness_pt: f32,
    /// Right-to-left binding (Hebrew book: front cover / page 1 opens on the left of the spread).
    pub rtl_binding: bool,
    /// Enable geometric creep compensation (shifts inner signature pages toward spine).
    pub creep_compensation: bool,
}

impl Default for ImpositionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            signature_size: 16,
            paper_thickness_pt: 0.28,
            rtl_binding: true,
            creep_compensation: true,
        }
    }
}

/// One side (front or back) of a physical press sheet containing two imposed logical pages.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImposedSheetSide {
    pub signature_index: usize,
    pub sheet_index_in_signature: usize,
    pub is_front: bool,
    /// 0-based logical page index placed on the left half of the press sheet (`None` = blank pad).
    pub left_page_index: Option<usize>,
    /// 0-based logical page index placed on the right half of the press sheet (`None` = blank pad).
    pub right_page_index: Option<usize>,
    /// Horizontal creep shift (in pt) applied to each half toward the spine (positive magnitude).
    pub creep_offset_pt: f32,
}

/// Complete imposition plan for a document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImpositionPlan {
    pub total_logical_pages: usize,
    pub padded_pages: usize,
    pub signature_size: usize,
    pub signature_count: usize,
    pub sheet_width_pt: f32,
    pub sheet_height_pt: f32,
    pub rtl_binding: bool,
    pub sides: Vec<ImposedSheetSide>,
}

/// Computes the horizontal creep compensation offset (in points) for a sheet within a signature.
///
/// Outer sheet (`sheet_index_in_signature == 0`) has `0.0` creep; the innermost sheet receives
/// `(sheets_per_sig - 1) * paper_thickness_pt` inward shift toward the spine so fore-edge
/// margins align after guillotine trimming.
#[inline]
pub fn compute_creep_offset(
    sheet_index_in_signature: usize,
    sheets_per_signature: usize,
    paper_thickness_pt: f32,
) -> f32 {
    if sheets_per_signature <= 1 || !paper_thickness_pt.is_finite() || paper_thickness_pt <= 0.0 {
        return 0.0;
    }
    let clamped_idx = sheet_index_in_signature.min(sheets_per_signature - 1);
    clamped_idx as f32 * paper_thickness_pt
}

/// Computes a 2-up saddle-stitch / multi-signature imposition plan for `total_pages`.
pub fn compute_imposition_plan(
    total_pages: usize,
    page_width_pt: f32,
    page_height_pt: f32,
    config: &ImpositionConfig,
) -> ImpositionPlan {
    let logical = total_pages.max(1);
    let sig_size = if config.signature_size == 0 {
        logical.div_ceil(4) * 4
    } else {
        config.signature_size.div_ceil(4) * 4
    };
    let padded_pages = logical.div_ceil(sig_size) * sig_size;
    let signature_count = padded_pages / sig_size;
    let sheets_per_sig = sig_size / 4;

    let mut sides = Vec::with_capacity(signature_count * sheets_per_sig * 2);

    for sig_idx in 0..signature_count {
        let base = sig_idx * sig_size;
        for sheet_idx in 0..sheets_per_sig {
            let creep = if config.creep_compensation {
                compute_creep_offset(sheet_idx, sheets_per_sig, config.paper_thickness_pt)
            } else {
                0.0
            };

            // Classic 2-up signature pairing (0-based within signature):
            // Front: outer-high (sig_size - 1 - 2*s), outer-low (2*s)
            // Back:  inner-low  (2*s + 1),           inner-high (sig_size - 2 - 2*s)
            let p_outer_low = base + 2 * sheet_idx;
            let p_outer_high = base + (sig_size - 1 - 2 * sheet_idx);
            let p_inner_low = base + 2 * sheet_idx + 1;
            let p_inner_high = base + (sig_size - 2 - 2 * sheet_idx);

            let opt = |p: usize| if p < total_pages { Some(p) } else { None };

            // In LTR: Front has [outer_high | outer_low], Back has [inner_low | inner_high]
            // In RTL (Hebrew book): Page 1 (outer_low) is on the LEFT of the front sheet,
            // outer_high is on the RIGHT; Back has [inner_high | inner_low].
            let (front_left, front_right, back_left, back_right) = if config.rtl_binding {
                (
                    opt(p_outer_low),
                    opt(p_outer_high),
                    opt(p_inner_high),
                    opt(p_inner_low),
                )
            } else {
                (
                    opt(p_outer_high),
                    opt(p_outer_low),
                    opt(p_inner_low),
                    opt(p_inner_high),
                )
            };

            sides.push(ImposedSheetSide {
                signature_index: sig_idx,
                sheet_index_in_signature: sheet_idx,
                is_front: true,
                left_page_index: front_left,
                right_page_index: front_right,
                creep_offset_pt: creep,
            });
            sides.push(ImposedSheetSide {
                signature_index: sig_idx,
                sheet_index_in_signature: sheet_idx,
                is_front: false,
                left_page_index: back_left,
                right_page_index: back_right,
                creep_offset_pt: creep,
            });
        }
    }

    ImpositionPlan {
        total_logical_pages: total_pages,
        padded_pages,
        signature_size: sig_size,
        signature_count,
        sheet_width_pt: page_width_pt * 2.0,
        sheet_height_pt: page_height_pt,
        rtl_binding: config.rtl_binding,
        sides,
    }
}

/// Applies an [`ImpositionPlan`] to rendered [`PageLayoutBox`]es, producing 2-up press-sheet
/// [`PageLayoutBox`]es with creep offsets applied toward the center spine.
pub fn impose_layout_pages(
    pages: &[PageLayoutBox],
    config: &ImpositionConfig,
) -> (ImpositionPlan, Vec<PageLayoutBox>) {
    let (pw, ph) = pages
        .first()
        .map(|p| (p.dimensions.width, p.dimensions.height))
        .unwrap_or((595.28, 841.89));

    let plan = compute_imposition_plan(pages.len(), pw, ph, config);
    let mut imposed_pages = Vec::with_capacity(plan.sides.len());

    for (sheet_side_idx, side) in plan.sides.iter().enumerate() {
        let mut sheet = PageLayoutBox::new(
            sheet_side_idx,
            format!("{}", sheet_side_idx + 1),
            PhysicalRect::new(0.0, 0.0, plan.sheet_width_pt, plan.sheet_height_pt),
            Vec::new(),
            None,
        );

        let place_half = |sheet: &mut PageLayoutBox, page_opt: Option<usize>, is_left_half: bool| {
            let Some(p_idx) = page_opt else {
                return;
            };
            let Some(src) = pages.get(p_idx) else {
                return;
            };
            let base_x = if is_left_half { 0.0 } else { pw };
            // Creep shifts toward the center spine (x = pw):
            // Left half shifts RIGHT (+creep), Right half shifts LEFT (-creep).
            let creep_dx = if is_left_half {
                side.creep_offset_pt
            } else {
                -side.creep_offset_pt
            };
            let dx = base_x + creep_dx;

            for frame in &src.frames {
                let mut f = frame.clone();
                f.rect.x += dx;
                sheet.frames.push(f);
            }
            for &(x1, y1, x2, y2) in &src.column_rules {
                sheet.column_rules.push((x1 + dx, y1, x2 + dx, y2));
            }
            for img in &src.images {
                let mut im = img.clone();
                im.rect.x += dx;
                sheet.images.push(im);
            }
            for orn in &src.ornaments {
                let mut o = orn.clone();
                o.rect.x += dx;
                sheet.ornaments.push(o);
            }
        };

        place_half(&mut sheet, side.left_page_index, true);
        place_half(&mut sheet, side.right_page_index, false);
        imposed_pages.push(sheet);
    }

    (plan, imposed_pages)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_16_page_signature_rtl_imposition_and_creep() {
        let cfg = ImpositionConfig {
            enabled: true,
            signature_size: 16,
            paper_thickness_pt: 0.5,
            rtl_binding: true,
            creep_compensation: true,
        };
        let plan = compute_imposition_plan(16, 400.0, 600.0, &cfg);
        assert_eq!(plan.signature_count, 1);
        assert_eq!(plan.sides.len(), 8); // 4 sheets * 2 sides

        // Sheet 0 front (RTL): Page 1 (idx 0) on left, Page 16 (idx 15) on right, creep = 0
        assert_eq!(plan.sides[0].left_page_index, Some(0));
        assert_eq!(plan.sides[0].right_page_index, Some(15));
        assert_eq!(plan.sides[0].creep_offset_pt, 0.0);

        // Sheet 0 back (RTL): Page 15 (idx 14) on left, Page 2 (idx 1) on right
        assert_eq!(plan.sides[1].left_page_index, Some(14));
        assert_eq!(plan.sides[1].right_page_index, Some(1));

        // Innermost sheet (sheet 3): creep = 3 * 0.5 = 1.5pt
        assert!((plan.sides[6].creep_offset_pt - 1.5).abs() < 1e-5);
    }

    #[test]
    fn test_impose_layout_pages_applies_spine_creep_shift() {
        let mut p1 = PageLayoutBox::new(0, "א".to_string(), PhysicalRect::new(0.0, 0.0, 400.0, 600.0), vec![], None);
        p1.frames.push(tok_typeset::TextFrameBox {
            frame_id: "f1".into(),
            flow_id: "main".into(),
            rect: PhysicalRect::new(50.0, 50.0, 300.0, 500.0),
            lines: vec![],
        });
        let p2 = PageLayoutBox::new(1, "ב".to_string(), PhysicalRect::new(0.0, 0.0, 400.0, 600.0), vec![], None);
        let mut p3 = PageLayoutBox::new(2, "ג".to_string(), PhysicalRect::new(0.0, 0.0, 400.0, 600.0), vec![], None);
        p3.frames.push(tok_typeset::TextFrameBox {
            frame_id: "f3".into(),
            flow_id: "main".into(),
            rect: PhysicalRect::new(50.0, 50.0, 300.0, 500.0),
            lines: vec![],
        });
        let p4 = PageLayoutBox::new(3, "ד".to_string(), PhysicalRect::new(0.0, 0.0, 400.0, 600.0), vec![], None);

        let cfg = ImpositionConfig {
            enabled: true,
            signature_size: 4,
            paper_thickness_pt: 0.5,
            rtl_binding: true,
            creep_compensation: true,
        };
        let (plan, imposed) = impose_layout_pages(&[p1, p2, p3, p4], &cfg);
        assert_eq!(plan.sides.len(), 2);
        assert_eq!(imposed[0].dimensions.width, 800.0);
        // Single sheet -> creep = 0
        assert_eq!(imposed[0].frames[0].rect.x, 50.0);
    }
}
