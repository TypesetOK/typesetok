//! OpenType / TrueType Font Management and Shaping Engine for TypesetOK.
//!
//! Provides:
//! - Embedded OFL Hebrew fonts: Frank Ruhl Libre (default body) and Noto Rashi
//!   Hebrew, the same glyphs the UI draws (built from assets/fonts/ui by
//!   scripts/build-engine-fonts.py), plus Noto Serif Hebrew (cantillation and
//!   meteg) and David Libre (Latin) as fallbacks.
//! - Dynamic font loading and registration (system fonts or project fonts).
//! - OpenType text shaping with rustybuzz (HarfBuzz-compatible) supporting GPOS mark/mkmk,
//!   GSUB ligatures, and font fallback.
//! - Accurate typographic metrics extraction (ascender, descender, cap-height, upem).

use crate::shaper::{ShapedRun, TextShaper};
use rustybuzz::Face;
use std::collections::BTreeMap;
use std::sync::Arc;

pub static EMBEDDED_FRANK_RUHL_LIBRE: &[u8] =
    include_bytes!("../../../assets/fonts/FrankRuhlLibre-Regular.ttf");
pub static EMBEDDED_FRANK_RUHL_LIBRE_BOLD: &[u8] =
    include_bytes!("../../../assets/fonts/FrankRuhlLibre-Bold.ttf");
pub static EMBEDDED_NOTO_RASHI_HEBREW: &[u8] =
    include_bytes!("../../../assets/fonts/NotoRashiHebrew-Regular.ttf");
pub static EMBEDDED_NOTO_RASHI_HEBREW_BOLD: &[u8] =
    include_bytes!("../../../assets/fonts/NotoRashiHebrew-Bold.ttf");
pub static EMBEDDED_NOTO_SERIF_HEBREW: &[u8] =
    include_bytes!("../../../assets/fonts/NotoSerifHebrew-Regular.ttf");
pub static EMBEDDED_DAVID_LIBRE: &[u8] =
    include_bytes!("../../../assets/fonts/DavidLibre-Regular.ttf");

/// Family used when a style names no font, or one that is not registered.
pub const DEFAULT_FAMILY: &str = "Frank Ruhl Libre";

/// Suffix of the registered bold (700) face of a family: "Frank Ruhl Libre Bold".
pub const BOLD_SUFFIX: &str = " Bold";

#[derive(Debug, Clone)]
pub struct FontMetrics {
    pub units_per_em: u16,
    pub ascender: i16,
    pub descender: i16,
    pub line_gap: i16,
    pub cap_height: Option<i16>,
    pub x_height: Option<i16>,
    pub x_min: i16,
    pub y_min: i16,
    pub x_max: i16,
    pub y_max: i16,
}

#[derive(Clone)]
pub struct FontData {
    pub family_name: String,
    pub raw_bytes: Arc<Vec<u8>>,
    pub metrics: FontMetrics,
}

impl std::fmt::Debug for FontData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FontData")
            .field("family_name", &self.family_name)
            .field("bytes_len", &self.raw_bytes.len())
            .field("metrics", &self.metrics)
            .finish()
    }
}

impl FontData {
    pub fn from_bytes(family_name: impl Into<String>, bytes: Vec<u8>) -> Result<Self, String> {
        let face = ttf_parser::Face::parse(&bytes, 0)
            .map_err(|e| format!("Failed to parse font: {}", e))?;
        if face.units_per_em() == 0 {
            return Err("Failed to parse font: unitsPerEm is zero".to_string());
        }

        let bbox = face.global_bounding_box();
        let metrics = FontMetrics {
            units_per_em: face.units_per_em(),
            ascender: face.ascender(),
            descender: face.descender(),
            line_gap: face.line_gap(),
            cap_height: face.capital_height(),
            x_height: face.x_height(),
            x_min: bbox.x_min,
            y_min: bbox.y_min,
            x_max: bbox.x_max,
            y_max: bbox.y_max,
        };

        Ok(Self {
            family_name: family_name.into(),
            raw_bytes: Arc::new(bytes),
            metrics,
        })
    }

    pub fn has_glyph(&self, ch: char) -> bool {
        ttf_parser::Face::parse(&self.raw_bytes, 0)
            .map(|face| face.glyph_index(ch).is_some())
            .unwrap_or(false)
    }

    /// Parses a shaping face over this font's bytes.
    pub fn shaping_face(&self) -> Option<Face<'_>> {
        Face::from_slice(&self.raw_bytes, 0)
    }
}

/// Which font shaped a run (see [`ShapingSession::shape`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontChoice {
    /// The requested family (after alias / default resolution).
    Primary,
    /// The n-th fallback family, used because the primary lacked glyphs.
    Fallback(usize),
    /// No usable font: metric heuristics, glyph ids are code points.
    Heuristic,
}

/// Fonts resolved and parsed once, for shaping many runs.
pub struct ShapingSession<'a> {
    primary: Option<(&'a FontData, Face<'a>)>,
    fallbacks: Vec<(&'a FontData, Face<'a>)>,
}

impl<'a> ShapingSession<'a> {
    fn missing_glyphs(face: &Face<'_>, text: &str) -> usize {
        text.chars()
            .filter(|c| !c.is_whitespace() && !c.is_control())
            .filter(|c| face.glyph_index(*c).is_none())
            .count()
    }

    /// Shapes `text` with the primary font, or with the first fallback that
    /// covers more of it when the primary lacks glyphs.
    pub fn shape(&self, text: &str, font_size_pt: f32, is_rtl: bool) -> (ShapedRun, FontChoice) {
        self.shape_with_features(text, font_size_pt, is_rtl, &[])
    }

    /// Shapes `text` with OpenType feature settings, falling back if glyphs are missing.
    pub fn shape_with_features(
        &self,
        text: &str,
        font_size_pt: f32,
        is_rtl: bool,
        extra_features: &[rustybuzz::Feature],
    ) -> (ShapedRun, FontChoice) {
        if text.is_empty() {
            return (
                ShapedRun {
                    glyphs: Vec::new(),
                    total_width_pt: 0.0,
                    font_size_pt,
                    is_rtl,
                },
                FontChoice::Primary,
            );
        }

        let primary_missing = self
            .primary
            .as_ref()
            .map(|(_, face)| Self::missing_glyphs(face, text));
        if primary_missing != Some(0) {
            // Fewest missing glyphs wins; ties go to the earlier fallback.
            let best = self
                .fallbacks
                .iter()
                .enumerate()
                .map(|(i, (_, face))| (Self::missing_glyphs(face, text), i))
                .min();
            if let Some((missing, i)) = best {
                if primary_missing.is_none_or(|p| missing < p) {
                    let face = &self.fallbacks[i].1;
                    return (
                        TextShaper::shape_with_face_and_features(
                            face,
                            text,
                            font_size_pt,
                            is_rtl,
                            extra_features,
                        ),
                        FontChoice::Fallback(i),
                    );
                }
            }
        }

        match &self.primary {
            Some((_, primary)) => (
                TextShaper::shape_with_face_and_features(
                    primary,
                    text,
                    font_size_pt,
                    is_rtl,
                    extra_features,
                ),
                FontChoice::Primary,
            ),
            None => (
                TextShaper::shape_fallback(text, font_size_pt, is_rtl),
                FontChoice::Heuristic,
            ),
        }
    }

    /// The registered family name of the font behind `choice`.
    pub fn family_of(&self, choice: FontChoice) -> Option<&'a str> {
        let font = match choice {
            FontChoice::Primary => self.primary.as_ref().map(|(f, _)| *f),
            FontChoice::Fallback(i) => self.fallbacks.get(i).map(|(f, _)| *f),
            FontChoice::Heuristic => None,
        };
        font.map(|f| f.family_name.as_str())
    }

    /// The choice spaces between words are drawn with: the primary font, or
    /// the first fallback when there is no primary.
    pub fn space_choice(&self) -> FontChoice {
        if self.primary.is_some() {
            FontChoice::Primary
        } else if !self.fallbacks.is_empty() {
            FontChoice::Fallback(0)
        } else {
            FontChoice::Heuristic
        }
    }

    /// Glyph id of the space character in the font of [`Self::space_choice`].
    pub fn space_glyph_id(&self) -> Option<u32> {
        self.primary
            .as_ref()
            .or(self.fallbacks.first())
            .and_then(|(_, face)| face.glyph_index(' '))
            .map(|g| u32::from(g.0))
    }
}

#[derive(Debug, Clone)]
pub struct FontManager {
    // BTreeMap: lookups that fall back to "any font" must be deterministic.
    fonts: BTreeMap<String, Arc<FontData>>,
    /// family -> weight -> registered face (see [`FontManager::face_for`]).
    weights: BTreeMap<String, BTreeMap<u16, String>>,
    default_hebrew_family: String,
    fallback_families: Vec<String>,
}

impl Default for FontManager {
    fn default() -> Self {
        let mut mgr = Self::new();
        for (family, bytes) in [
            ("Frank Ruhl Libre", EMBEDDED_FRANK_RUHL_LIBRE),
            ("Frank Ruhl Libre Bold", EMBEDDED_FRANK_RUHL_LIBRE_BOLD),
            ("Noto Rashi Hebrew", EMBEDDED_NOTO_RASHI_HEBREW),
            ("Noto Rashi Hebrew Bold", EMBEDDED_NOTO_RASHI_HEBREW_BOLD),
            ("Noto Serif Hebrew", EMBEDDED_NOTO_SERIF_HEBREW),
            ("David Libre", EMBEDDED_DAVID_LIBRE),
        ] {
            let font = FontData::from_bytes(family, bytes.to_vec())
                .unwrap_or_else(|e| panic!("embedded {family} must be a valid font: {e}"));
            mgr.register_font(font);
        }
        for family in ["Frank Ruhl Libre", "Noto Rashi Hebrew"] {
            mgr.register_weight(family, 400, family);
            mgr.register_weight(family, 700, &format!("{family}{BOLD_SUFFIX}"));
        }

        mgr.set_default_hebrew_family(DEFAULT_FAMILY);
        // Noto Serif Hebrew covers cantillation and meteg, which Frank Ruhl Libre
        // and David Libre lack; David Libre covers Latin, which Noto Serif lacks.
        mgr.set_fallback_families(&["Noto Serif Hebrew", "David Libre"]);

        // Names used by older documents and styles.
        for (alias, family) in [
            ("Taamey Frank CLM", "Frank Ruhl Libre"),
            ("Frank Ruehl CLM", "Frank Ruhl Libre"),
            ("Vilna", "Frank Ruhl Libre"),
            ("Rashi", "Noto Rashi Hebrew"),
            ("David CLM", "David Libre"),
            ("David", "David Libre"),
        ] {
            mgr.alias_font(alias, family);
        }

        mgr
    }
}

impl FontManager {
    pub fn new() -> Self {
        Self {
            fonts: BTreeMap::new(),
            weights: BTreeMap::new(),
            default_hebrew_family: String::new(),
            fallback_families: Vec::new(),
        }
    }

    pub fn register_font(&mut self, font: FontData) {
        self.fonts.insert(font.family_name.clone(), Arc::new(font));
    }

    pub fn alias_font(&mut self, alias: &str, target_family: &str) {
        if let Some(target) = self.fonts.get(target_family).cloned() {
            self.fonts.insert(alias.to_string(), target);
        }
    }

    pub fn set_default_hebrew_family(&mut self, family: &str) {
        self.default_hebrew_family = family.to_string();
    }

    pub fn set_fallback_family(&mut self, family: &str) {
        self.set_fallback_families(&[family]);
    }

    /// Fallbacks in priority order, tried when the requested font lacks glyphs.
    pub fn set_fallback_families(&mut self, families: &[&str]) {
        self.fallback_families = families.iter().map(|f| f.to_string()).collect();
    }

    /// Resolves a family (or alias), falling back to the default Hebrew family.
    pub fn font_ref(&self, family: &str) -> Option<&Arc<FontData>> {
        self.fonts.get(family).or_else(|| {
            if self.default_hebrew_family.is_empty() {
                self.fonts.values().next()
            } else {
                self.fonts.get(&self.default_hebrew_family)
            }
        })
    }

    /// Registers `face` (a registered family name) as `family` at `weight`.
    /// A family's own regular face is weight 400 unless registered otherwise.
    pub fn register_weight(&mut self, family: &str, weight: u16, face: &str) {
        self.weights
            .entry(family.to_string())
            .or_default()
            .insert(weight, face.to_string());
    }

    /// The registered face to shape `family` at `weight` with: aliases and the
    /// default are resolved first, then the closest weight is chosen as CSS
    /// does (up to 500: lighter first, then heavier; above: heavier first).
    pub fn face_for(&self, family: &str, weight: u16) -> String {
        let Some(font) = self.font_ref(family) else {
            return family.to_string();
        };
        let base = font.family_name.as_str();
        let Some(faces) = self.weights.get(base) else {
            return base.to_string();
        };
        let lighter = || faces.range(..=weight).next_back();
        let heavier = || faces.range(weight..).next();
        let chosen = if weight <= 500 {
            lighter().or_else(heavier)
        } else {
            heavier().or_else(lighter)
        };
        chosen.map_or_else(|| base.to_string(), |(_, face)| face.clone())
    }

    /// Exact lookup of a registered family or alias, without default fallback.
    pub fn font_exact(&self, family: &str) -> Option<&Arc<FontData>> {
        self.fonts.get(family)
    }

    pub fn get_font(&self, family: &str) -> Option<Arc<FontData>> {
        self.font_ref(family).cloned()
    }

    pub fn default_font(&self) -> Option<Arc<FontData>> {
        self.get_font(&self.default_hebrew_family)
    }

    /// Resolves and parses the fonts needed to shape text in `font_family`.
    pub fn session(&self, font_family: &str) -> ShapingSession<'_> {
        fn parse(font: &Arc<FontData>) -> Option<(&FontData, Face<'_>)> {
            let font: &FontData = font;
            font.shaping_face().map(|face| (font, face))
        }
        let primary = self.font_ref(font_family).and_then(parse);
        let mut fallbacks: Vec<(&FontData, Face<'_>)> = Vec::new();
        for family in &self.fallback_families {
            let Some(font) = self.fonts.get(family) else {
                continue;
            };
            let font_ptr: &FontData = font;
            let seen = primary
                .iter()
                .chain(fallbacks.iter())
                .any(|(f, _)| std::ptr::eq(*f, font_ptr));
            if !seen {
                if let Some(entry) = parse(font) {
                    fallbacks.push(entry);
                }
            }
        }
        ShapingSession { primary, fallbacks }
    }

    /// Shapes text with the requested font family, automatically falling back for missing glyphs.
    pub fn shape_text(
        &self,
        text: &str,
        font_family: &str,
        font_size_pt: f32,
        is_rtl: bool,
    ) -> ShapedRun {
        self.session(font_family)
            .shape(text, font_size_pt, is_rtl)
            .0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedded_font_manager() {
        let mgr = FontManager::default();
        let noto = mgr
            .get_font("Noto Serif Hebrew")
            .expect("Noto Serif Hebrew must exist");
        assert_eq!(noto.metrics.units_per_em, 1000);
        assert!(noto.metrics.ascender > 0);
        assert!(noto.metrics.descender < 0);

        let david = mgr.get_font("David Libre").expect("David Libre must exist");
        assert_eq!(david.metrics.units_per_em, 2048);
    }

    #[test]
    fn test_shape_hebrew_with_real_font() {
        let mgr = FontManager::default();
        let text = "בְּרֵאשִׁית בָּרָא אֱלֹהִים";
        let run = mgr.shape_text(text, "Noto Serif Hebrew", 12.0, true);

        assert!(run.is_rtl);
        assert!(!run.glyphs.is_empty());
        assert!(run.total_width_pt > 0.0);

        // Combining marks should have 0 or near-0 advance in OpenType mark positioning
        let zero_adv = run.glyphs.iter().filter(|g| g.x_advance == 0.0).count();
        assert!(
            zero_adv > 0,
            "Combining niqqud marks must have 0 advance in GPOS"
        );
    }

    #[test]
    fn test_font_fallback_for_latin_in_hebrew_font() {
        let mgr = FontManager::default();
        let text = "TypesetOK 2026";
        let run = mgr.shape_text(text, "Noto Serif Hebrew", 12.0, false);

        assert!(!run.glyphs.is_empty());
        assert!(run.total_width_pt > 0.0);
    }

    #[test]
    fn session_reports_the_font_that_shaped_the_run() {
        let mgr = FontManager::default();
        let session = mgr.session("Noto Serif Hebrew");
        let (_, choice) = session.shape("שלום", 12.0, true);
        assert_eq!(choice, FontChoice::Primary);
        assert_eq!(session.family_of(choice), Some("Noto Serif Hebrew"));
        let (_, choice) = session.shape("Typeset", 12.0, false);
        assert!(matches!(choice, FontChoice::Fallback(_)));
        assert_eq!(session.family_of(choice), Some("David Libre"));

        // Aliases resolve to the real family name.
        let session = mgr.session("David CLM");
        assert_eq!(session.family_of(FontChoice::Primary), Some("David Libre"));
        assert!(session.space_glyph_id().is_some());
    }

    #[test]
    fn unknown_family_without_default_is_deterministic() {
        let mut mgr = FontManager::new();
        for name in ["Zeta", "Alpha", "Mid"] {
            mgr.register_font(
                FontData::from_bytes(name, EMBEDDED_NOTO_SERIF_HEBREW.to_vec()).unwrap(),
            );
        }
        for _ in 0..5 {
            assert_eq!(mgr.get_font("missing").unwrap().family_name, "Alpha");
        }
    }

    #[test]
    fn empty_manager_uses_heuristic_shaper() {
        let mgr = FontManager::new();
        let (run, choice) = mgr.session("x").shape("אב", 10.0, true);
        assert_eq!(choice, FontChoice::Heuristic);
        assert_eq!(run.glyphs.len(), 2);
    }

    #[test]
    fn invalid_font_bytes_are_rejected() {
        assert!(FontData::from_bytes("bad", vec![0, 1, 2, 3]).is_err());
    }

    #[test]
    fn ui_fonts_are_the_defaults_and_old_names_resolve_to_them() {
        let mgr = FontManager::default();
        assert_eq!(mgr.default_font().unwrap().family_name, "Frank Ruhl Libre");
        assert_eq!(
            mgr.get_font("unknown family").unwrap().family_name,
            "Frank Ruhl Libre"
        );
        for (name, family) in [
            ("Frank Ruhl Libre", "Frank Ruhl Libre"),
            ("Noto Rashi Hebrew", "Noto Rashi Hebrew"),
            ("Vilna", "Frank Ruhl Libre"),
            ("Taamey Frank CLM", "Frank Ruhl Libre"),
            ("Frank Ruehl CLM", "Frank Ruhl Libre"),
            ("Rashi", "Noto Rashi Hebrew"),
            ("David CLM", "David Libre"),
        ] {
            let session = mgr.session(name);
            assert_eq!(
                session.family_of(FontChoice::Primary),
                Some(family),
                "{name}"
            );
        }
    }

    #[test]
    fn face_for_picks_the_closest_shipped_weight() {
        let mgr = FontManager::default();
        for (family, weight, face) in [
            ("Frank Ruhl Libre", 400, "Frank Ruhl Libre"),
            ("Frank Ruhl Libre", 700, "Frank Ruhl Libre Bold"),
            // CSS matching: 500 and below prefer lighter, 600 and above heavier.
            ("Frank Ruhl Libre", 300, "Frank Ruhl Libre"),
            ("Frank Ruhl Libre", 500, "Frank Ruhl Libre"),
            ("Frank Ruhl Libre", 600, "Frank Ruhl Libre Bold"),
            ("Frank Ruhl Libre", 900, "Frank Ruhl Libre Bold"),
            ("Vilna", 700, "Frank Ruhl Libre Bold"),
            ("Rashi", 700, "Noto Rashi Hebrew Bold"),
            ("Rashi", 400, "Noto Rashi Hebrew"),
            // No bold David Libre is shipped: the regular face is used.
            ("David CLM", 700, "David Libre"),
            ("unknown", 700, "Frank Ruhl Libre Bold"),
        ] {
            assert_eq!(mgr.face_for(family, weight), face, "{family} {weight}");
        }
        let bold = mgr.get_font("Frank Ruhl Libre Bold").unwrap();
        let face = ttf_parser::Face::parse(&bold.raw_bytes, 0).unwrap();
        assert_eq!(face.weight().to_number(), 700);
    }

    #[test]
    fn fallback_order_covers_cantillation_and_latin() {
        let mgr = FontManager::default();

        // Pointed text and Latin are in Frank Ruhl Libre itself.
        let session = mgr.session("Frank Ruhl Libre");
        for text in ["בְּרֵאשִׁית", "TypesetOK 2026", "תשפ״ד"] {
            let (_, choice) = session.shape(text, 12.0, true);
            assert_eq!(choice, FontChoice::Primary, "{text}");
        }
        // Cantillation is not, so the word goes to Noto Serif Hebrew.
        let (run, choice) = session.shape("בְּרֵאשִׁ֖ית", 12.0, true);
        assert_eq!(session.family_of(choice), Some("Noto Serif Hebrew"));
        assert!(run.total_width_pt > 0.0);

        // Noto Rashi Hebrew has cantillation itself.
        let (_, choice) = mgr.session("Rashi").shape("בְּרֵאשִׁ֖ית", 12.0, true);
        assert_eq!(choice, FontChoice::Primary);

        // Latin in Noto Serif Hebrew skips to David Libre, past the primary.
        let session = mgr.session("Noto Serif Hebrew");
        let (_, choice) = session.shape("Typeset", 12.0, false);
        assert_eq!(session.family_of(choice), Some("David Libre"));
    }
}
