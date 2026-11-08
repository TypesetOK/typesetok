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
    /// Optional OpenType feature settings for the paragraph (e.g. "calt", "ss01", "ss04").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opentype_features: Option<Vec<OpenTypeFeatureSetting>>,
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
            opentype_features: None,
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
        if let Some(v) = &patch.opentype_features {
            self.opentype_features = Some(v.clone());
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CharacterStyle {
    pub id: String,
    pub name: String,
    pub font_family: Option<String>,
    pub font_size_pt: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_scale: Option<f32>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub tracking_em: Option<f32>,
    pub color: Option<Color>,
    pub baseline_shift_pt: Option<f32>,
    pub masoretic_variant: Option<MasoreticLetterVariant>,
    pub opentype_features: Option<Vec<OpenTypeFeatureSetting>>,
}

impl CharacterStyle {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            font_family: None,
            font_size_pt: None,
            font_scale: None,
            bold: None,
            italic: None,
            tracking_em: None,
            color: None,
            baseline_shift_pt: None,
            masoretic_variant: None,
            opentype_features: None,
        }
    }

    /// Pre-configured Character Style for Literae Majusculae (אות רבתי).
    /// Scales font size by ~1.30x and sets masoretic variant to Rabbatai (with 'titl' OpenType feature).
    pub fn rabbatai(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            font_family: None,
            font_size_pt: None,
            font_scale: Some(1.30),
            bold: None,
            italic: None,
            tracking_em: None,
            color: None,
            baseline_shift_pt: Some(0.0),
            masoretic_variant: Some(MasoreticLetterVariant::Rabbatai),
            opentype_features: Some(vec![OpenTypeFeatureSetting::new("titl", 1)]),
        }
    }

    /// Pre-configured Character Style for Literae Minusculae (אות זעירא).
    /// Scales font size by ~0.72x and sets masoretic variant to Zeira (with 'smcp' OpenType feature).
    pub fn zeira(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            font_family: None,
            font_size_pt: None,
            font_scale: Some(0.72),
            bold: None,
            italic: None,
            tracking_em: None,
            color: None,
            baseline_shift_pt: Some(0.0),
            masoretic_variant: Some(MasoreticLetterVariant::Zeira),
            opentype_features: Some(vec![OpenTypeFeatureSetting::new("smcp", 1)]),
        }
    }

    /// Pre-configured Character Style for Literae Suspensae (אות תלויה).
    /// Scales font size by ~0.72x, elevates baseline, and sets masoretic variant to Teluyah (with 'sups' OpenType feature).
    pub fn teluyah(
        id: impl Into<String>,
        name: impl Into<String>,
        baseline_shift_pt: Option<f32>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            font_family: None,
            font_size_pt: None,
            font_scale: Some(0.72),
            bold: None,
            italic: None,
            tracking_em: None,
            color: None,
            baseline_shift_pt,
            masoretic_variant: Some(MasoreticLetterVariant::Teluyah),
            opentype_features: Some(vec![OpenTypeFeatureSetting::new("sups", 1)]),
        }
    }

    /// Pre-configured Character Style using a dedicated OpenType feature (such as cvXX or ssXX) for Torani fonts.
    pub fn with_opentype_feature(
        id: impl Into<String>,
        name: impl Into<String>,
        tag: impl Into<String>,
        value: u32,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            font_family: None,
            font_size_pt: None,
            font_scale: None,
            bold: None,
            italic: None,
            tracking_em: None,
            color: None,
            baseline_shift_pt: None,
            masoretic_variant: None,
            opentype_features: Some(vec![OpenTypeFeatureSetting::new(tag, value)]),
        }
    }

    /// Convert this `CharacterStyle` into a `CharacterFormat`.
    pub fn to_format(&self) -> CharacterFormat {
        CharacterFormat {
            character_style_id: Some(self.id.clone()),
            font_family: self.font_family.clone(),
            font_size_pt: self.font_size_pt,
            font_scale: self.font_scale,
            font_weight: self.bold.map(|b| if b { FONT_WEIGHT_BOLD } else { FONT_WEIGHT_REGULAR }),
            italic: self.italic,
            underline: None,
            tracking_em: self.tracking_em,
            color: self.color.clone(),
            baseline_shift_pt: self.baseline_shift_pt,
            masoretic_variant: self.masoretic_variant,
            opentype_features: self.opentype_features.clone(),
        }
    }
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opentype_features: Option<Vec<OpenTypeFeatureSetting>>,
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
    /// Proportional font scale relative to paragraph font size (e.g. `0.8` = 80%).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_scale: Option<f32>,
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

    /// Merges `other` on top of `self`, returning a combined `CharacterFormat`.
    pub fn merged_with(&self, other: &CharacterFormat) -> CharacterFormat {
        CharacterFormat {
            character_style_id: other
                .character_style_id
                .clone()
                .or_else(|| self.character_style_id.clone()),
            font_family: other
                .font_family
                .clone()
                .or_else(|| self.font_family.clone()),
            font_size_pt: other.font_size_pt.or(self.font_size_pt),
            font_scale: other.font_scale.or(self.font_scale),
            font_weight: other.font_weight.or(self.font_weight),
            italic: other.italic.or(self.italic),
            underline: other.underline.or(self.underline),
            tracking_em: other.tracking_em.or(self.tracking_em),
            color: other.color.clone().or_else(|| self.color.clone()),
            baseline_shift_pt: other.baseline_shift_pt.or(self.baseline_shift_pt),
            masoretic_variant: other.masoretic_variant.or(self.masoretic_variant),
            opentype_features: other
                .opentype_features
                .clone()
                .or_else(|| self.opentype_features.clone()),
        }
    }

    /// Resolves this format against available `CharacterStyle` definitions.
    /// If `character_style_id` matches an existing style, the style's properties are inherited
    /// and any explicit direct overrides in `self` take precedence.
    pub fn resolve_with_styles(&self, styles: &[CharacterStyle]) -> CharacterFormat {
        if let Some(ref sid) = self.character_style_id {
            if let Some(cs) = styles.iter().find(|s| &s.id == sid) {
                return cs.to_format().merged_with(self);
            }
        }
        self.clone()
    }
}

/// Non-destructive Live GREP styling rule (מנוע GREP חי).
///
/// Evaluates a regular expression against paragraph text at layout/render time
/// and dynamically applies a `CharacterStyle` or inline `CharacterFormat` without
/// mutating the underlying document text.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GrepRule {
    pub id: String,
    pub name: String,
    pub pattern: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_style_id: Option<String>,
    #[serde(default)]
    pub format: CharacterFormat,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope_flow_id: Option<String>,
}

impl GrepRule {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        pattern: impl Into<String>,
        format: CharacterFormat,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            pattern: pattern.into(),
            target_style_id: None,
            format,
            enabled: true,
            scope_flow_id: None,
        }
    }

    pub fn with_target_style(mut self, style_id: impl Into<String>) -> Self {
        let id = style_id.into();
        self.target_style_id = Some(id.clone());
        if self.format.character_style_id.is_none() {
            self.format.character_style_id = Some(id);
        }
        self
    }

    pub fn with_scope_flow(mut self, flow_id: impl Into<String>) -> Self {
        self.scope_flow_id = Some(flow_id.into());
        self
    }

    /// Resolves the effective `CharacterFormat` for this rule, combining any linked
    /// `CharacterStyle` with the rule's direct `format` overrides.
    pub fn effective_format(&self, char_styles: &[CharacterStyle]) -> CharacterFormat {
        let style_id = self
            .target_style_id
            .as_deref()
            .or(self.format.character_style_id.as_deref());
        let mut base = CharacterFormat::default();
        if let Some(sid) = style_id {
            if let Some(cs) = char_styles.iter().find(|s| s.id == sid) {
                base.character_style_id = Some(cs.id.clone());
                base.font_family = cs.font_family.clone();
                base.font_size_pt = cs.font_size_pt;
                base.font_scale = cs.font_scale;
                if let Some(bold) = cs.bold {
                    base.font_weight = Some(if bold {
                        FONT_WEIGHT_BOLD
                    } else {
                        FONT_WEIGHT_REGULAR
                    });
                }
                base.italic = cs.italic;
                base.tracking_em = cs.tracking_em;
                base.color = cs.color.clone();
                base.baseline_shift_pt = cs.baseline_shift_pt;
                base.masoretic_variant = cs.masoretic_variant;
                base.opentype_features = cs.opentype_features.clone();
            }
        }
        base.merged_with(&self.format)
    }

    /// Standard Torah & Hebrew DTP presets for one-click activation.
    pub fn builtin_presets() -> Vec<Self> {
        vec![
            Self::new(
                "grep-parens-citations",
                "הקטנת מראי מקומות בסוגריים (80%)",
                r"\([^()\n]{1,120}\)",
                CharacterFormat {
                    font_scale: Some(0.80),
                    color: Some(Color::DeviceCmyk {
                        c: 0.0,
                        m: 0.0,
                        y: 0.0,
                        k: 0.65,
                    }),
                    ..Default::default()
                },
            ),
            Self::new(
                "grep-acronyms-gershayim",
                "הדגשת ראשי תיבות (גרשיים)",
                r#"[\u0590-\u05FF]+(?:"|״|'')[\u0590-\u05FF]+"#,
                CharacterFormat {
                    font_weight: Some(FONT_WEIGHT_BOLD),
                    ..Default::default()
                },
            ),
            Self::new(
                "grep-talmud-openings",
                "הדגשת מילות פתיחה בגמרא (אמר רבא, תנו רבנן)",
                r"^(?:אמר רבא|אמר אביי|תנו רבנן|איתמר|בעא מיניה|תא שמע|מתני׳|מתני'|גמ׳|גמ')\b",
                CharacterFormat {
                    font_weight: Some(FONT_WEIGHT_BOLD),
                    ..Default::default()
                },
            ),
            Self::new(
                "grep-section-markers",
                "הדגשת מספרי סעיפים ואותיות (אות א', סעיף ב')",
                r"^(?:אות|סעיף|סימן|הלכה|ענף)\s+[\u05D0-\u05EA]{1,4}[׳'״.]?",
                CharacterFormat {
                    font_weight: Some(FONT_WEIGHT_BOLD),
                    ..Default::default()
                },
            ),
            Self::new(
                "grep-double-quotes",
                "עיצוב ציטוטים במירכאות כפולות",
                r#"(?:"[^"\n]{1,200}"|״[^״\n]{1,200}״)"#,
                CharacterFormat {
                    italic: Some(true),
                    ..Default::default()
                },
            ),
        ]
    }
}

/// Evaluates active Live GREP rules on `text` without mutating it, and merges
/// the resulting spans with `existing_spans` (explicit `existing_spans` win over
/// GREP rules where both specify the same property).
pub fn evaluate_live_grep_spans(
    text: &str,
    existing_spans: &[crate::spans::TextSpan],
    rules: &[GrepRule],
    char_styles: &[CharacterStyle],
    flow_id: Option<&str>,
) -> Vec<crate::spans::TextSpan> {
    if text.is_empty() {
        return Vec::new();
    }
    let active_rules: Vec<&GrepRule> = rules
        .iter()
        .filter(|r| {
            r.enabled
                && !r.pattern.is_empty()
                && match (&r.scope_flow_id, flow_id) {
                    (Some(scope), Some(fid)) if !scope.is_empty() && scope != "all" => scope == fid,
                    (Some(scope), None) if !scope.is_empty() && scope != "all" => false,
                    _ => true,
                }
        })
        .collect();

    let resolved_existing: Vec<crate::spans::TextSpan> = existing_spans
        .iter()
        .map(|s| {
            if s.format.character_style_id.is_some() {
                let mut resolved = s.clone();
                resolved.format = s.format.resolve_with_styles(char_styles);
                resolved
            } else {
                s.clone()
            }
        })
        .collect();

    if active_rules.is_empty() {
        return resolved_existing;
    }

    // Build byte-offset -> char-offset lookup table for O(log N) / O(1) conversion
    let mut byte_to_char = vec![0usize; text.len() + 1];
    let mut char_count = 0usize;
    for (byte_idx, ch) in text.char_indices() {
        byte_to_char[byte_idx] = char_count;
        char_count += 1;
        let next_byte = byte_idx + ch.len_utf8();
        byte_to_char[next_byte] = char_count;
    }

    let mut combined_spans: Vec<crate::spans::TextSpan> = Vec::new();
    for rule in active_rules {
        let fmt = rule.effective_format(char_styles);
        if fmt.is_empty() {
            continue;
        }
        if let Ok(re) = regex::Regex::new(&rule.pattern) {
            for mat in re.find_iter(text) {
                let start_char = byte_to_char[mat.start()];
                let end_char = byte_to_char[mat.end()];
                if start_char < end_char {
                    combined_spans.push(crate::spans::TextSpan::new(
                        start_char,
                        end_char,
                        fmt.clone(),
                    ));
                }
            }
        }
    }

    if combined_spans.is_empty() {
        return resolved_existing;
    }

    // Normalize GREP spans first, then layer explicit spans on top so user formatting wins
    let mut normalized_grep = crate::spans::normalize_spans(combined_spans, char_count);
    for exp in resolved_existing {
        normalized_grep.push(exp);
    }
    crate::spans::normalize_spans(normalized_grep, char_count)
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

    #[test]
    fn test_live_grep_rule_presets_and_non_destructive_evaluation() {
        let presets = GrepRule::builtin_presets();
        assert!(presets.len() >= 5);

        let text = "אמר רבא כתבו הרמב\"ם והרשב\"א (שבת דף כג ע״א) בדין נר חנוכה.";
        let spans = evaluate_live_grep_spans(text, &[], &presets, &[], Some("main"));

        // Verify original text was not mutated and spans cover:
        // 1. "אמר רבא" (chars 0..7) -> bold 700
        // 2. "הרמב\"ם", "והרשב\"א", and "ע״א" -> bold 700
        // 3. "(שבת דף כג ע״א)" -> font_scale 0.80 & gray CMYK
        assert!(spans
            .iter()
            .any(|s| s.start == 0 && s.end == 7 && s.format.font_weight == Some(FONT_WEIGHT_BOLD)));
        assert!(spans
            .iter()
            .any(|s| s.format.font_scale == Some(0.80) && s.format.color.is_some()));
        let acronym_spans: Vec<_> = spans
            .iter()
            .filter(|s| s.start > 7 && s.format.font_weight == Some(FONT_WEIGHT_BOLD))
            .collect();
        assert_eq!(acronym_spans.len(), 3);
    }

    #[test]
    fn test_character_style_masoretic_variants_and_resolution() {
        let cs_rabbatai = CharacterStyle::rabbatai("rabbatai", "אות רבתי");
        assert_eq!(cs_rabbatai.font_scale, Some(1.30));
        assert_eq!(cs_rabbatai.baseline_shift_pt, Some(0.0));
        assert_eq!(cs_rabbatai.masoretic_variant, Some(MasoreticLetterVariant::Rabbatai));
        assert_eq!(
            cs_rabbatai.opentype_features,
            Some(vec![OpenTypeFeatureSetting::new("titl", 1)])
        );

        let cs_zeira = CharacterStyle::zeira("zeira", "אות זעירא");
        assert_eq!(cs_zeira.font_scale, Some(0.72));
        assert_eq!(cs_zeira.baseline_shift_pt, Some(0.0));
        assert_eq!(cs_zeira.masoretic_variant, Some(MasoreticLetterVariant::Zeira));
        assert_eq!(
            cs_zeira.opentype_features,
            Some(vec![OpenTypeFeatureSetting::new("smcp", 1)])
        );

        let cs_teluyah = CharacterStyle::teluyah("teluyah", "אות תלויה", Some(4.5));
        assert_eq!(cs_teluyah.font_scale, Some(0.72));
        assert_eq!(cs_teluyah.baseline_shift_pt, Some(4.5));
        assert_eq!(cs_teluyah.masoretic_variant, Some(MasoreticLetterVariant::Teluyah));
        assert_eq!(
            cs_teluyah.opentype_features,
            Some(vec![OpenTypeFeatureSetting::new("sups", 1)])
        );

        let cs_cv = CharacterStyle::with_opentype_feature("cv01", "Torah Variant 1", "cv01", 1);
        assert_eq!(
            cs_cv.opentype_features,
            Some(vec![OpenTypeFeatureSetting::new("cv01", 1)])
        );

        // Resolution via CharacterFormat
        let mut fmt = CharacterFormat::default();
        fmt.character_style_id = Some("teluyah".to_string());
        let resolved = fmt.resolve_with_styles(&[cs_teluyah]);
        assert_eq!(resolved.font_scale, Some(0.72));
        assert_eq!(resolved.baseline_shift_pt, Some(4.5));
        assert_eq!(resolved.masoretic_variant, Some(MasoreticLetterVariant::Teluyah));
        assert_eq!(
            resolved.opentype_features,
            Some(vec![OpenTypeFeatureSetting::new("sups", 1)])
        );

        // Resolution via evaluate_live_grep_spans on existing spans
        let existing = vec![crate::spans::TextSpan::new(0, 1, fmt)];
        let evaluated = evaluate_live_grep_spans("שמע", &existing, &[], &[CharacterStyle::teluyah("teluyah", "אות תלויה", Some(4.5))], None);
        assert_eq!(evaluated.len(), 1);
        assert_eq!(evaluated[0].format.font_scale, Some(0.72));
        assert_eq!(evaluated[0].format.baseline_shift_pt, Some(4.5));
    }
}
