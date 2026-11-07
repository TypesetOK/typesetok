//! Pre-press page boxes (MediaBox, BleedBox, TrimBox, CropBox) and Vector Printer Marks.

use pdf_writer::Content;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PrePressPageBoxes {
    pub trim_box: [f32; 4], // [llx, lly, urx, ury]
    pub bleed_box: [f32; 4],
    pub media_box: [f32; 4],
    pub crop_box: [f32; 4],
}

impl PrePressPageBoxes {
    /// Computes PDF page boxes given trimmed dimensions, bleed (default 3mm = 8.504 pt),
    /// and slug margin for crop marks (default 10mm = 28.346 pt).
    pub fn new(page_width_pt: f32, page_height_pt: f32, bleed_pt: f32, slug_pt: f32) -> Self {
        let w = if page_width_pt.is_finite() && page_width_pt > 0.0 {
            page_width_pt
        } else {
            595.28
        };
        let h = if page_height_pt.is_finite() && page_height_pt > 0.0 {
            page_height_pt
        } else {
            841.89
        };
        let b = if bleed_pt.is_finite() && bleed_pt >= 0.0 {
            bleed_pt
        } else {
            0.0
        };
        let mut s = if slug_pt.is_finite() && slug_pt >= 0.0 {
            slug_pt
        } else {
            0.0
        };

        if s < b {
            s = b;
        }

        let trim_llx = s;
        let trim_lly = s;
        let trim_urx = s + w;
        let trim_ury = s + h;

        let bleed_llx = trim_llx - b;
        let bleed_lly = trim_lly - b;
        let bleed_urx = trim_urx + b;
        let bleed_ury = trim_ury + b;

        let media_width = w + 2.0 * s;
        let media_height = h + 2.0 * s;

        Self {
            trim_box: [trim_llx, trim_lly, trim_urx, trim_ury],
            bleed_box: [bleed_llx, bleed_lly, bleed_urx, bleed_ury],
            media_box: [0.0, 0.0, media_width, media_height],
            crop_box: [0.0, 0.0, media_width, media_height],
        }
    }

    /// Emits PDF vector content stream operators to draw precision crop marks in the slug area.
    pub fn draw_crop_marks(&self, content: &mut Content) {
        let [x1, y1, x2, y2] = self.trim_box;
        let mark_len = 14.17; // 5mm
        let mark_offset = 5.67; // 2mm away from trim line

        content.set_line_width(0.25);
        content.set_stroke_cmyk(0.0, 0.0, 0.0, 1.0);

        // Bottom-left corner marks
        content.move_to(x1 - mark_offset - mark_len, y1);
        content.line_to(x1 - mark_offset, y1);
        content.stroke();
        content.move_to(x1, y1 - mark_offset - mark_len);
        content.line_to(x1, y1 - mark_offset);
        content.stroke();

        // Bottom-right corner marks
        content.move_to(x2 + mark_offset, y1);
        content.line_to(x2 + mark_offset + mark_len, y1);
        content.stroke();
        content.move_to(x2, y1 - mark_offset - mark_len);
        content.line_to(x2, y1 - mark_offset);
        content.stroke();

        // Top-left corner marks
        content.move_to(x1 - mark_offset - mark_len, y2);
        content.line_to(x1 - mark_offset, y2);
        content.stroke();
        content.move_to(x1, y2 + mark_offset);
        content.line_to(x1, y2 + mark_offset + mark_len);
        content.stroke();

        // Top-right corner marks
        content.move_to(x2 + mark_offset, y2);
        content.line_to(x2 + mark_offset + mark_len, y2);
        content.stroke();
        content.move_to(x2, y2 + mark_offset);
        content.line_to(x2, y2 + mark_offset + mark_len);
        content.stroke();
    }

    /// Emits PDF vector content stream operators to draw registration crosshairs (צלבי רישום)
    /// centered along all four slug edges using 100% all-plates registration ink.
    pub fn draw_registration_marks(&self, content: &mut Content) {
        let [x1, y1, x2, y2] = self.trim_box;
        let mid_x = (x1 + x2) * 0.5;
        let mid_y = (y1 + y2) * 0.5;
        let offset = 11.34; // 4mm outside trim
        let arm = 4.5;

        content.set_line_width(0.25);
        content.set_stroke_cmyk(0.0, 0.0, 0.0, 1.0);

        let centers = [
            (mid_x, y1 - offset),
            (mid_x, y2 + offset),
            (x1 - offset, mid_y),
            (x2 + offset, mid_y),
        ];
        for (cx, cy) in centers {
            content.move_to(cx - arm, cy);
            content.line_to(cx + arm, cy);
            content.stroke();
            content.move_to(cx, cy - arm);
            content.line_to(cx, cy + arm);
            content.stroke();
        }
    }
}
