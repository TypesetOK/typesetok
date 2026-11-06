//! Traditional Biblical and Masoretic Hebrew Typography (טיפוגרפיה מסורתית של התנ"ך והמסורה).
//!
//! Provides:
//! - Support for Literae Majusculae (אותיות רבתי, e.g. ב' רבתי בראשית, ע' ו-ד' בשמע),
//!   Literae Minusculae (אותיות זעירות, e.g. ה' זעירא בבהבראם, א' זעירא בויקרא),
//!   and Literae Suspensae (אותיות תלויות, e.g. ע' תלויה, נ' תלויה במנשה).
//! - Accurate handling and shaping for Inverted Nun (נו"ן הפוכה `U+05C6`).
//! - Dedicated OpenType feature mapping for rare cantillation marks (טעמי מקרא נדירים)
//!   and Torani fonts (e.g. `calt`, `ss01`, `ss02`, `ss04`, `cv01`-`cv20`, `titl`, `sups`, `smcp`).

use rustybuzz::ttf_parser::Tag;
use rustybuzz::Feature;
use tok_core::styles::{MasoreticLetterVariant, OpenTypeFeatureSetting};

/// Codepoint for Inverted Nun (נו"ן הפוכה).
pub const NUN_HAFUKHA: char = '\u{05C6}';

/// Checks if character is a rare or specialized Biblical cantillation mark.
pub fn is_rare_cantillation_mark(c: char) -> bool {
    matches!(
        c as u32,
        0x0592 | // Segolta (סְגוֹלְתָּא)
        0x0593 | // Shalshelet (שַׁלְשֶׁלֶת)
        0x059F | // Pazer Gadol / Qarne Farah (קַרְנֵי פָרָה)
        0x05A6 | // Merkha Kefula (מֵרְכָא כְּפוּלָה)
        0x05AA | // Yerach Ben Yomo (יֶרַח בֶּן יוֹמוֹ)
        0x05AB | // Ole Ve-Yored (עֹלֶה וְיוֹרֵד)
        0x05A2 | // Atnah Hafukh / Iluy (אֶתְנַח הָפוּךְ)
        0x05AD | // Dehi (דְּחִי)
        0x05AE // Tsinorit (צִנּוֹרִית)
    )
}

/// Returns true if character is a poetic cantillation mark used in Sifrei Emet (טעמי אמ"ת: תהלים, משלי, איוב).
pub fn is_poetic_emet_accent(c: char) -> bool {
    matches!(c as u32, 0x05A2 | 0x05AB | 0x05AC | 0x05AD | 0x05AE)
}

/// Checks whether the text contains any rare cantillation marks or Masoretic special symbols.
pub fn has_rare_cantillation_or_masoretic_marks(text: &str) -> bool {
    text.chars()
        .any(|c| is_rare_cantillation_mark(c) || c == NUN_HAFUKHA)
}

/// Configuration for Biblical and Masoretic Hebrew Typography features.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MasoreticTypographyConfig {
    /// Enables OpenType contextual alternates (`calt`) for biblical mark positioning and collision resolution.
    pub enable_contextual_alternates: bool,
    /// Enables OpenType stylistic sets (`ss01`, `ss04`) for rare cantillation mark variants and Torani fonts.
    pub enable_rare_cantillation_alternates: bool,
    /// Enables OpenType stylistic sets for Sifrei Emet (`ss02`).
    pub enable_poetic_accents_emet: bool,
    /// Enables dedicated Inverted Nun alternates (`ss05` or `cv04`) if font supports them.
    pub enable_inverted_nun_feature: bool,
    /// Custom OpenType feature overrides.
    pub custom_features: Vec<(String, u32)>,
}

impl Default for MasoreticTypographyConfig {
    fn default() -> Self {
        Self {
            enable_contextual_alternates: true,
            enable_rare_cantillation_alternates: true,
            enable_poetic_accents_emet: false,
            enable_inverted_nun_feature: true,
            custom_features: Vec::new(),
        }
    }
}

impl MasoreticTypographyConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_poetic_accents(mut self, enabled: bool) -> Self {
        self.enable_poetic_accents_emet = enabled;
        self
    }

    pub fn with_custom_feature(mut self, tag: impl Into<String>, value: u32) -> Self {
        self.custom_features.push((tag.into(), value));
        self
    }

    /// Resolves active OpenType features for a given text string.
    pub fn resolve_features_for_text(&self, text: &str) -> Vec<Feature> {
        let mut features = Vec::new();

        // 1. Contextual Alternates (active by default in Biblical Hebrew shaping)
        if self.enable_contextual_alternates {
            features.push(Feature::new(Tag::from_bytes(b"calt"), 1, ..));
        }

        // 2. Rare Cantillation Alternates (ss01 for general accent positioning, ss04 for rare glyph variants)
        let has_rare = text.chars().any(is_rare_cantillation_mark);
        if self.enable_rare_cantillation_alternates && has_rare {
            features.push(Feature::new(Tag::from_bytes(b"ss01"), 1, ..));
            features.push(Feature::new(Tag::from_bytes(b"ss04"), 1, ..));
        }

        // 3. Poetic Accents for Sifrei Emet
        let has_emet = text.chars().any(is_poetic_emet_accent);
        if (self.enable_poetic_accents_emet || has_emet) && self.enable_rare_cantillation_alternates
        {
            features.push(Feature::new(Tag::from_bytes(b"ss02"), 1, ..));
        }

        // 4. Inverted Nun (U+05C6)
        if self.enable_inverted_nun_feature && text.contains(NUN_HAFUKHA) {
            features.push(Feature::new(Tag::from_bytes(b"ss05"), 1, ..));
        }

        // 5. Custom features
        for (tag_str, val) in &self.custom_features {
            if let Ok(bytes) = tag_str.as_bytes().try_into() {
                features.push(Feature::new(Tag::from_bytes(bytes), *val, ..));
            }
        }

        features
    }
}

/// Applies typographical rules for a Masoretic letter variant (אות רבתי, זעירא, תלויה).
///
/// Returns:
/// - `font_size_pt`: scaled font size (pt)
/// - `baseline_shift_pt`: vertical baseline shift (pt, positive = upward)
/// - `opentype_feature`: optional OpenType feature tag recommendation (e.g. "titl", "smcp", "sups")
pub fn resolve_masoretic_letter_styling(
    variant: MasoreticLetterVariant,
    base_font_size_pt: f32,
    explicit_baseline_shift_pt: Option<f32>,
) -> (f32, f32, Option<Feature>) {
    let scale = variant.font_size_scale();
    let effective_size = base_font_size_pt * scale;

    let baseline_shift = if let Some(explicit) = explicit_baseline_shift_pt {
        explicit
    } else {
        variant.baseline_shift_em() * base_font_size_pt
    };

    let ot_feature = variant.recommended_opentype_tag().map(|tag| {
        let bytes: [u8; 4] = tag.as_bytes().try_into().unwrap_or(*b"calt");
        Feature::new(Tag::from_bytes(&bytes), 1, ..)
    });

    (effective_size, baseline_shift, ot_feature)
}

/// Converts core `OpenTypeFeatureSetting` entries into rustybuzz `Feature`s.
pub fn convert_opentype_settings(settings: &[OpenTypeFeatureSetting]) -> Vec<Feature> {
    settings
        .iter()
        .filter_map(|s| {
            let bytes: Result<[u8; 4], _> = s.tag.as_bytes().try_into();
            bytes
                .ok()
                .map(|b| Feature::new(Tag::from_bytes(&b), s.value, ..))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tok_core::normalizer::HebrewNormalizer;

    #[test]
    fn test_rare_cantillation_detection() {
        assert!(is_rare_cantillation_mark('\u{0592}')); // Shalshelet
        assert!(is_rare_cantillation_mark('\u{059F}')); // Qarne Farah
        assert!(is_rare_cantillation_mark('\u{05A6}')); // Merkha Kefula
        assert!(is_rare_cantillation_mark('\u{05AA}')); // Yerach Ben Yomo
        assert!(!is_rare_cantillation_mark('\u{0591}')); // Etnahta (common)
        assert!(!is_rare_cantillation_mark('\u{05B0}')); // Sheva (niqqud)
    }

    #[test]
    fn test_nun_hafukha_detection_and_shaping_feature() {
        assert_eq!(NUN_HAFUKHA, '\u{05C6}');
        assert!(HebrewNormalizer::is_nun_hafukha(NUN_HAFUKHA));

        let text = "׆ וַיְהִי בִּנְסֹעַ הָאָרֹן ׆";
        let config = MasoreticTypographyConfig::default();
        let features = config.resolve_features_for_text(text);

        // Must include calt and ss05 for inverted nun
        let tags: Vec<String> = features
            .iter()
            .map(|f| String::from_utf8_lossy(&f.tag.to_bytes()).to_string())
            .collect();
        assert!(tags.contains(&"calt".to_string()));
        assert!(tags.contains(&"ss05".to_string()));
    }

    #[test]
    fn test_masoretic_letter_variant_styling() {
        // Rabbatai (ב' רבתי)
        let (r_size, r_shift, r_feat) =
            resolve_masoretic_letter_styling(MasoreticLetterVariant::Rabbatai, 12.0, None);
        assert!((r_size - 12.0 * 1.30).abs() < 1e-4);
        assert_eq!(r_shift, 0.0);
        assert_eq!(
            r_feat.map(|f| String::from_utf8_lossy(&f.tag.to_bytes()).to_string()),
            Some("titl".to_string())
        );

        // Zeira (ל' זעירא)
        let (z_size, z_shift, z_feat) =
            resolve_masoretic_letter_styling(MasoreticLetterVariant::Zeira, 12.0, None);
        assert!((z_size - 12.0 * 0.72).abs() < 1e-4);
        assert_eq!(z_shift, 0.0);
        assert_eq!(
            z_feat.map(|f| String::from_utf8_lossy(&f.tag.to_bytes()).to_string()),
            Some("smcp".to_string())
        );

        // Teluyah (ע' תלויה, נ' תלויה)
        let (t_size, t_shift, t_feat) =
            resolve_masoretic_letter_styling(MasoreticLetterVariant::Teluyah, 12.0, None);
        assert!((t_size - 12.0 * 0.72).abs() < 1e-4);
        assert!(
            t_shift > 0.0,
            "Suspended letter must have positive upward baseline shift"
        );
        assert_eq!(
            t_feat.map(|f| String::from_utf8_lossy(&f.tag.to_bytes()).to_string()),
            Some("sups".to_string())
        );
    }

    #[test]
    fn test_opentype_features_for_rare_cantillation() {
        // Text with Shalshelet and Yerach Ben Yomo
        let text = "וַיִּתְמַהְמָ֓הּ ... יֶרַח בֶּן יוֹמוֹ֪";
        let config = MasoreticTypographyConfig::default();
        let features = config.resolve_features_for_text(text);

        let tags: Vec<String> = features
            .iter()
            .map(|f| String::from_utf8_lossy(&f.tag.to_bytes()).to_string())
            .collect();
        assert!(tags.contains(&"calt".to_string()));
        assert!(tags.contains(&"ss01".to_string()));
        assert!(tags.contains(&"ss04".to_string()));
    }
}
