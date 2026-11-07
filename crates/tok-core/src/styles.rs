use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Color {
    Rgb {
        r: f32,
        g: f32,
        b: f32,
    },
    DeviceCmyk {
        c: f32,
        m: f32,
        y: f32,
        k: f32,
    },
    Spot {
        name: String,
        tint: f32,
        fallback_cmyk: (f32, f32, f32, f32),
    },
}

impl Default for Color {
    fn default() -> Self {
        // Standard DTP 100% black in CMYK
        Color::DeviceCmyk {
            c: 0.0,
            m: 0.0,
            y: 0.0,
            k: 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Progression {
    #[default]
    Rtl,
    Ltr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TextAlignment {
    Right,
    Left,
    Center,
    #[default]
    Justified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AhaltermStretchMode {
    Off,
    #[default]
    AutoVariable,
    StaticSwash,
}

fn default_true() -> bool {
    true
}

fn default_drop_cap_lines() -> usize {
    3
}

fn default_drop_cap_chars() -> usize {
    1
}

fn default_drop_cap_weight() -> u16 {
    FONT_WEIGHT_BOLD
}

fn default_drop_cap_margin() -> f32 {
    6.0
}

fn default_optical_factor() -> f32 {
    1.0
}

/// Optical Margin Alignment (Hanging Punctuation and Glyph Overhang Protrusion) configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpticalMarginConfig {
    pub enabled: bool,
    /// Scaling factor for optical overhang protrusion (1.0 = 100% nominal overhang).
    #[serde(default = "default_optical_factor")]
    pub factor: f32,
    /// Whether punctuation marks (hyphens, maqaf, geresh, gershayim, periods, commas, quotes) protrude.
    #[serde(default = "default_true")]
    pub protrude_punctuation: bool,
    /// Whether overhanging letterforms (e.g. curved/diagonal Hebrew and Latin glyphs) protrude.
    #[serde(default = "default_true")]
    pub protrude_glyphs: bool,
}

impl Default for OpticalMarginConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            factor: 1.0,
            protrude_punctuation: true,
            protrude_glyphs: true,
        }
    }
}

/// Constrained last-line balancing configuration for Knuth-Plass paragraph breaking.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct LastLineConstraintConfig {
    /// Minimum required width (in pt) for the final line of a multi-line paragraph.
    #[serde(default)]
    pub min_width_pt: f32,
    /// Minimum required width as a ratio `[0.0, 1.0)` of the line's target measure.
    #[serde(default)]
    pub min_width_ratio: f32,
    /// Minimum number of words on the final line of a multi-line paragraph (0 or 1 = unrestricted, 2 = prevent single-word runts).
    #[serde(default)]
    pub min_words: usize,
    /// Whether to penalize large slack disparities between the penultimate and final lines.
    #[serde(default)]
    pub balance_slack: bool,
}

/// Hebrew Opening Word / Drop Cap configuration ("חלון במילת פתיחה").
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DropCapConfig {
    pub enabled: bool,
    /// Number of lines the drop cap window spans vertically (default: 3).
    #[serde(default = "default_drop_cap_lines")]
    pub lines: usize,
    /// Number of characters/letters included if whole_word is false.
    #[serde(default = "default_drop_cap_chars")]
    pub characters_count: usize,
    /// If true, the entire first word is used as opening word (traditional Hebrew "חלון").
    #[serde(default = "default_true")]
    pub whole_word: bool,
    /// Font family for the drop cap (None = inherits paragraph font).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,
    /// Font weight (default: 700 bold).
    #[serde(default = "default_drop_cap_weight")]
    pub font_weight: u16,
    /// Margin / clearance (pt) around the drop cap window.
    #[serde(default = "default_drop_cap_margin")]
    pub margin_pt: f32,
    /// Explicit font size (pt). If None, automatically scaled to fit `lines * line_height`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_size_pt: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_scale: Option<f32>,
}

impl Default for DropCapConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            lines: 3,
            characters_count: 1,
            whole_word: true,
            font_family: None,
            font_weight: FONT_WEIGHT_BOLD,
            margin_pt: 6.0,
            font_size_pt: None,
        }
    }
}

pub const FONT_WEIGHT_REGULAR: u16 = 400;
pub const FONT_WEIGHT_BOLD: u16 = 700;

fn default_font_weight() -> u16 {
    FONT_WEIGHT_REGULAR
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParagraphStyle {
    pub id: String,
    pub name: String,
    pub font_family: String,
    /// CSS-style weight: 400 regular, 700 bold. Absent in older documents.
    #[serde(default = "default_font_weight")]
    pub font_weight: u16,
    pub font_size_pt: f32,
    pub line_height_pt: f32,
    pub space_before_pt: f32,
    pub space_after_pt: f32,
    pub first_line_indent_pt: f32,
    /// Indent on the side where lines start (right in RTL, left in LTR).
    /// Absent in documents written before Stage 11.
    #[serde(default)]
    pub indent_start_pt: f32,
    /// Indent on the side where lines end (left in RTL, right in LTR).
    #[serde(default)]
    pub indent_end_pt: f32,
    /// Base paragraph direction. `None` follows `DocumentMetadata::progression`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<Progression>,
    pub alignment: TextAlignment,
    pub ahalterm_stretch: AhaltermStretchMode,
    pub color: Color,
    pub keep_with_next: bool,
    pub orphan_control: u8,
    pub widow_control: u8,
    /// Optional drop cap or Hebrew opening word window.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drop_cap: Option<DropCapConfig>,
    /// Optional optical margin alignment (hanging punctuation & glyph overhangs).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub optical_margin: Option<OpticalMarginConfig>,
    /// Optional constrained last-line balancing rules for Knuth-Plass.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_line_constraint: Option<LastLineConstraintConfig>,
}

impl Default for ParagraphStyle {
    fn default() -> Self {
        Self {
            id: "default-body".to_string(),
            name: "גוף הטקסט".to_string(),
            font_family: "Frank Ruhl Libre".to_string(),
            font_weight: FONT_WEIGHT_REGULAR,
            font_size_pt: 11.0,
            line_height_pt: 14.5,
            space_before_pt: 0.0,
            space_after_pt: 2.0,
            first_line_indent_pt: 0.0,
            indent_start_pt: 0.0,
            indent_end_pt: 0.0,
            direction: None,
            alignment: TextAlignment::Justified,
            ahalterm_stretch: AhaltermStretchMode::AutoVariable,
            color: Color::default(),
            keep_with_next: false,
            orphan_control: 2,
            widow_control: 2,
            drop_cap: None,
            optical_margin: None,
            last_line_constraint: None,
        }
    }
}

impl ParagraphStyle {
    /// Applies a local paragraph override on top of this (named) style.
    /// Only the fields the patch sets are changed.
    pub fn apply_patch(&mut self, patch: &StylePatch) {
        if let Some(v) = &patch.font_family {
            self.font_family = v.clone();
        }
        if let Some(v) = patch.font_weight {
            self.font_weight = v;
        }
        if let Some(v) = patch.font_size_pt {
            self.font_size_pt = v;
        }
        if let Some(v) = patch.line_height_pt {
            self.line_height_pt = v;
        }
        if let Some(v) = patch.space_before_pt {
            self.space_before_pt = v;
        }
        if let Some(v) = patch.space_after_pt {
            self.space_after_pt = v;
        }
        if let Some(v) = patch.first_line_indent_pt {
            self.first_line_indent_pt = v;
        }
        if let Some(v) = patch.indent_start_pt {
            self.indent_start_pt = v;
        }
        if let Some(v) = patch.indent_end_pt {
            self.indent_end_pt = v;
        }
        if let Some(v) = patch.direction {
            self.direction = Some(v);
        }
        if let Some(v) = patch.alignment {
            self.alignment = v;
        }
        if let Some(v) = &patch.color {
            self.color = v.clone();
        }
        if let Some(v) = &patch.drop_cap {
            self.drop_cap = Some(v.clone());
        }
        if let Some(v) = &patch.optical_margin {
            self.optical_margin = Some(v.clone());
        }
        if let Some(v) = &patch.last_line_constraint {
            self.last_line_constraint = Some(v.clone());
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CharacterStyle {
    pub id: String,
    pub name: String,
    pub font_family: Option<String>,
    pub font_size_pt: Option<f32>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub tracking_em: Option<f32>,
    pub color: Option<Color>,
    pub baseline_shift_pt: Option<f32>,
    pub masoretic_variant: Option<MasoreticLetterVariant>,
    pub opentype_features: Option<Vec<OpenTypeFeatureSetting>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_scale: Option<f32>,
}

/// OpenType feature setting tag (e.g. "calt", "ss01", "ss04", "cv04") and value (0 or 1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenTypeFeatureSetting {
    pub tag: String,
    pub value: u32,
}

impl OpenTypeFeatureSetting {
    pub fn new(tag: impl Into<String>, value: u32) -> Self {
        Self {
            tag: tag.into(),
            value,
        }
    }
}

/// Masoretic letter variation in Biblical Hebrew typography (טיפוגרפיה מסורתית של התנ"ך והמסורה).
///
/// Traditional Masoretic codices feature three distinct letter size/position modifications:
/// 1. `Rabbatai` (אותיות רבתי / Literae Majusculae) — enlarged letters (e.g. ב' בראשית, ע' ו-ד' בשמע).
/// 2. `Zeira` (אותיות זעירות / Literae Minusculae) — reduced letters (e.g. ה' בבהבראם, א' בויקרא).
/// 3. `Teluyah` (אותיות תלויות / Literae Suspensae) — suspended / elevated letters positioned above baseline (e.g. ע' תלויה ביער מיער, נ' תלויה במנשה).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum MasoreticLetterVariant {
    #[default]
    Regular,
    /// Literae Majusculae (אות רבתי) - typical size scaling ~1.25x - 1.35x.
    Rabbatai,
    /// Literae Minusculae (אות זעירא) - typical size scaling ~0.70x - 0.75x.
    Zeira,
    /// Literae Suspensae (אות תלויה) - smaller size (~0.72x) elevated above baseline.
    Teluyah,
}

impl MasoreticLetterVariant {
    /// Recommended font scale relative to current font size.
    pub fn font_size_scale(&self) -> f32 {
        match self {
            Self::Regular => 1.0,
            Self::Rabbatai => 1.30,
            Self::Zeira => 0.72,
            Self::Teluyah => 0.72,
        }
    }

    /// Baseline shift in em units (positive shifts upwards).
    pub fn baseline_shift_em(&self) -> f32 {
        match self {
            Self::Regular | Self::Rabbatai | Self::Zeira => 0.0,
            Self::Teluyah => 0.38,
        }
    }

    /// OpenType stylistic set or character variant tag if supported by Torani OpenType fonts.
    pub fn recommended_opentype_tag(&self) -> Option<&'static str> {
        match self {
            Self::Regular => None,
            Self::Rabbatai => Some("titl"),
            Self::Zeira => Some("smcp"),
            Self::Teluyah => Some("sups"),
        }
    }
}

/// A local paragraph-level override (direct formatting) applied on top of the
/// paragraph's named style. Patches are applied in order; later ones win.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct StylePatch {
    pub font_family: Option<String>,
    pub font_size_pt: Option<f32>,
    pub line_height_pt: Option<f32>,
    pub alignment: Option<TextAlignment>,
    pub tracking_em: Option<f32>,
    pub color: Option<Color>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_weight: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub space_before_pt: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub space_after_pt: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_line_indent_pt: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub indent_start_pt: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub indent_end_pt: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<Progression>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drop_cap: Option<DropCapConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub optical_margin: Option<OpticalMarginConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_line_constraint: Option<LastLineConstraintConfig>,
}

impl StylePatch {
    pub fn is_empty(&self) -> bool {
        *self == StylePatch::default()
    }
}

/// Direct character formatting carried by a [`TextSpan`]. Every field is
/// optional: `None` means "inherit from the paragraph".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CharacterFormat {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub character_style_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_size_pt: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_weight: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub italic: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub underline: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tracking_em: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub baseline_shift_pt: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub masoretic_variant: Option<MasoreticLetterVariant>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opentype_features: Option<Vec<OpenTypeFeatureSetting>>,
}

impl CharacterFormat {
    pub fn is_empty(&self) -> bool {
        *self == CharacterFormat::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paragraph_style_without_weight_loads_as_regular() {
        let mut json = serde_json::to_value(ParagraphStyle::default()).unwrap();
        assert_eq!(json["font_weight"], 400);
        json.as_object_mut().unwrap().remove("font_weight");
        let style: ParagraphStyle = serde_json::from_value(json).unwrap();
        assert_eq!(style.font_weight, FONT_WEIGHT_REGULAR);
    }

    #[test]
    fn pre_stage_11_style_loads_without_indents_or_direction() {
        let mut json = serde_json::to_value(ParagraphStyle::default()).unwrap();
        let obj = json.as_object_mut().unwrap();
        obj.remove("indent_start_pt");
        obj.remove("indent_end_pt");
        obj.remove("direction");
        let style: ParagraphStyle = serde_json::from_value(json).unwrap();
        assert_eq!(style.indent_start_pt, 0.0);
        assert_eq!(style.indent_end_pt, 0.0);
        assert_eq!(style.direction, None);
    }

    #[test]
    fn legacy_style_patch_json_still_parses() {
        let legacy = r#"{"font_family":null,"font_size_pt":12.0,"line_height_pt":null,
            "alignment":"Center","tracking_em":null,"color":null}"#;
        let patch: StylePatch = serde_json::from_str(legacy).unwrap();
        assert_eq!(patch.font_size_pt, Some(12.0));
        assert_eq!(patch.alignment, Some(TextAlignment::Center));
        assert_eq!(patch.direction, None);
        assert_eq!(patch.font_weight, None);
    }

    #[test]
    fn apply_patch_only_overrides_set_fields() {
        let mut style = ParagraphStyle::default();
        let patch = StylePatch {
            font_size_pt: Some(14.0),
            font_weight: Some(FONT_WEIGHT_BOLD),
            alignment: Some(TextAlignment::Center),
            indent_start_pt: Some(18.0),
            direction: Some(Progression::Ltr),
            ..StylePatch::default()
        };
        style.apply_patch(&patch);
        assert_eq!(style.font_size_pt, 14.0);
        assert_eq!(style.font_weight, FONT_WEIGHT_BOLD);
        assert_eq!(style.alignment, TextAlignment::Center);
        assert_eq!(style.indent_start_pt, 18.0);
        assert_eq!(style.direction, Some(Progression::Ltr));
        // Untouched fields keep the named style's values.
        assert_eq!(style.font_family, "Frank Ruhl Libre");
        assert_eq!(style.line_height_pt, 14.5);
        assert!(StylePatch::default().is_empty());
        assert!(!patch.is_empty());
    }

    #[test]
    fn character_format_serializes_only_set_fields() {
        let fmt = CharacterFormat {
            font_weight: Some(FONT_WEIGHT_BOLD),
            italic: Some(true),
            ..CharacterFormat::default()
        };
        let json = serde_json::to_string(&fmt).unwrap();
        assert_eq!(json, r#"{"font_weight":700,"italic":true}"#);
        let back: CharacterFormat = serde_json::from_str(&json).unwrap();
        assert_eq!(back, fmt);
        assert!(CharacterFormat::default().is_empty());
    }

    #[test]
    fn test_drop_cap_serialization_defaults() {
        let dc = DropCapConfig::default();
        assert!(dc.enabled);
        assert_eq!(dc.lines, 3);
        assert_eq!(dc.characters_count, 1);
        assert!(dc.whole_word);
        assert_eq!(dc.font_weight, FONT_WEIGHT_BOLD);
        assert_eq!(dc.margin_pt, 6.0);
        assert_eq!(dc.font_size_pt, None);

        let json = serde_json::to_string(&dc).unwrap();
        let deserialized: DropCapConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(dc, deserialized);
    }

    #[test]
    fn test_drop_cap_apply_patch_overrides() {
        let mut style = ParagraphStyle::default();
        assert_eq!(style.drop_cap, None);

        let dc = DropCapConfig {
            enabled: true,
            lines: 2,
            characters_count: 1,
            whole_word: false,
            font_family: Some("Frank Ruhl Libre".to_string()),
            font_weight: 800,
            margin_pt: 8.5,
            font_size_pt: Some(36.0),
        };

        let patch = StylePatch {
            drop_cap: Some(dc.clone()),
            ..Default::default()
        };

        style.apply_patch(&patch);
        assert_eq!(style.drop_cap, Some(dc));
    }

    #[test]
    fn test_masoretic_variant_scaling_and_serialization() {
        assert_eq!(MasoreticLetterVariant::Rabbatai.font_size_scale(), 1.30);
        assert_eq!(MasoreticLetterVariant::Zeira.font_size_scale(), 0.72);
        assert_eq!(MasoreticLetterVariant::Teluyah.font_size_scale(), 0.72);
        assert_eq!(MasoreticLetterVariant::Teluyah.baseline_shift_em(), 0.38);
        assert_eq!(MasoreticLetterVariant::Regular.baseline_shift_em(), 0.0);

        let fmt = CharacterFormat {
            masoretic_variant: Some(MasoreticLetterVariant::Teluyah),
            baseline_shift_pt: Some(4.5),
            opentype_features: Some(vec![OpenTypeFeatureSetting::new("ss01", 1)]),
            ..Default::default()
        };
        let json = serde_json::to_string(&fmt).unwrap();
        let parsed: CharacterFormat = serde_json::from_str(&json).unwrap();
        assert_eq!(fmt, parsed);
    }
}