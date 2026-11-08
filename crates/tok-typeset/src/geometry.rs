use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PhysicalPoint {
    pub x: f32, // in DTP points (pt)
    pub y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PhysicalRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl PhysicalRect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn contains_point(&self, pt: PhysicalPoint) -> bool {
        pt.x >= self.x
            && pt.x <= self.x + self.width
            && pt.y >= self.y
            && pt.y <= self.y + self.height
    }

    pub fn a4_portrait() -> Self {
        // A4: 210mm x 297mm = 595.28 pt x 841.89 pt
        Self {
            x: 0.0,
            y: 0.0,
            width: 595.28,
            height: 841.89,
        }
    }
}

/// Font index marking a glyph whose font is unknown (heuristic shaping);
/// renderers resolve such glyphs through their `character`.
pub const UNKNOWN_FONT: u16 = u16::MAX;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GlyphBox {
    pub glyph_id: u32,
    /// UTF-8 byte offset of the glyph's cluster in the paragraph text.
    pub cluster: u32,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub character: Option<char>,
    /// Index into the owning line's [`LineBox::fonts`]; glyph ids are only
    /// meaningful in that font.
    #[serde(default)]
    pub font_index: u16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LineBox {
    pub line_index: usize,
    pub paragraph_id: Option<tok_core::id::NodeId>,
    pub baseline_y: f32,
    pub height: f32,
    pub width: f32,
    /// Glyphs in visual (left-to-right) order.
    pub glyphs: Vec<GlyphBox>,
    /// Line text in logical order.
    pub text: String,
    pub is_rtl: bool,
    /// Registered font family names referenced by `GlyphBox::font_index`.
    /// Empty for lines produced without font information.
    #[serde(default)]
    pub fonts: Vec<String>,
    /// Whether this line represents an opening word / drop cap window.
    #[serde(default)]
    pub is_drop_cap: bool,
    /// Horizontal offset of this line relative to the text frame (in pt).
    #[serde(default)]
    pub x_offset: f32,
}

impl LineBox {
    /// Font family of a glyph on this line, if known.
    pub fn font_of(&self, glyph: &GlyphBox) -> Option<&str> {
        self.fonts
            .get(usize::from(glyph.font_index))
            .map(String::as_str)
    }

    /// Horizontal extent `(left, right)` of the inked line relative to its
    /// frame; `(0, width)` when the line has no glyphs.
    pub fn horizontal_extent(&self) -> (f32, f32) {
        let mut extent: Option<(f32, f32)> = None;
        for g in &self.glyphs {
            let (l, r) = (g.x, g.x + g.width);
            extent = Some(match extent {
                Some((el, er)) => (el.min(l), er.max(r)),
                None => (l, r),
            });
        }
        extent.unwrap_or((0.0, self.width))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextFrameBox {
    pub frame_id: String,
    pub flow_id: String,
    pub rect: PhysicalRect,
    pub lines: Vec<LineBox>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BreakToken {
    pub section_index: usize,
    pub paragraph_index: usize,
    pub char_offset: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paragraph_id: Option<tok_core::id::NodeId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PageLayoutBox {
    pub page_index: usize,
    pub page_number_gematria: String,
    pub dimensions: PhysicalRect,
    pub frames: Vec<TextFrameBox>,
    pub break_token: Option<BreakToken>,
    /// Indicates whether this page is an intentionally injected blank page (e.g. Blank Verso Page
    /// to force an opening chapter/siman onto an odd Recto page).
    #[serde(default)]
    pub is_blank: bool,
}

impl PageLayoutBox {
    pub fn new(
        page_index: usize,
        page_number_gematria: String,
        dimensions: PhysicalRect,
        frames: Vec<TextFrameBox>,
        break_token: Option<BreakToken>,
    ) -> Self {
        Self {
            page_index,
            page_number_gematria,
            dimensions,
            frames,
            break_token,
            is_blank: false,
        }
    }

    pub fn blank(
        page_index: usize,
        page_number_gematria: String,
        dimensions: PhysicalRect,
    ) -> Self {
        Self {
            page_index,
            page_number_gematria,
            dimensions,
            frames: Vec::new(),
            break_token: None,
            is_blank: true,
        }
    }
}
