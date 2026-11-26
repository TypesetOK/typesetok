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
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmbeddingPermission {
    Installable,
    Editable,
    PreviewAndPrint,
    Restricted,
    BitmapOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
pub enum FontLoadError {
    #[error(
        "Web font formats (WOFF/WOFF2) are not supported for PDF/X prepress; supply a TTF font."
    )]
    RejectWoff,
    #[error("OpenType CFF (.otf) PostScript outlines are not yet supported; supply a TrueType-flavored (.ttf) font with a 'glyf' table.")]
    RejectCff,
    #[error("Invalid font binary: {0}")]
    InvalidFontBinary(String),
    #[error("Font OS/2 fsType=0x0002 restricts embedding (Restricted License); commercial PDF embedding prohibited.")]
    RestrictedLicense,
    #[error("Font OS/2 fsType indicates Bitmap-Only embedding, which is incompatible with vector PDF/X.")]
    BitmapOnlyEmbedding,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FontInspectionReport {
    pub family_name: String,
    pub postscript_name: Option<String>,
    pub weight: u16,
    pub is_bold: bool,
    pub is_italic: bool,
    pub glyph_count: u16,
    pub units_per_em: u16,
    pub fs_type: u16,
    pub embedding_permission: EmbeddingPermission,
    pub is_embeddable: bool,
    pub has_hebrew_letters: bool,
    pub hebrew_coverage_pct: f32,
    pub niqqud_coverage_pct: f32,
    pub taamim_coverage_pct: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissingFontDiagnostic {
    pub family_name: String,
    pub missing_family: bool,
    pub missing_hebrew_coverage: bool,
    pub missing_niqqud_coverage: bool,
    pub missing_taamim_coverage: bool,
    pub affected_paragraphs: usize,
}

const HEBREW_LETTERS_COUNT: usize = 27; // U+05D0..=U+05EA
const NIQQUD_CODEPOINTS: [char; 16] = [
    '\u{05B0}', '\u{05B1}', '\u{05B2}', '\u{05B3}', '\u{05B4}', '\u{05B5}', '\u{05B6}', '\u{05B7}',
    '\u{05B8}', '\u{05B9}', '\u{05BA}', '\u{05BB}', '\u{05BC}', '\u{05C1}', '\u{05C2}', '\u{05C7}',
];

fn extract_name_string(face: &ttf_parser::Face<'_>, target_id: u16) -> Option<String> {
    let mut fallback: Option<String> = None;
    for name in face.names() {
        if name.name_id != target_id {
            continue;
        }
        if name.is_unicode() {
            if let Some(s) = name.to_string() {
                let t = s.trim();
                if !t.is_empty() {
                    if name.platform_id == ttf_parser::PlatformId::Windows {
                        return Some(t.to_string());
                    }
                    if fallback.is_none() {
                        fallback = Some(t.to_string());
                    }
                }
            }
        } else if name.platform_id == ttf_parser::PlatformId::Macintosh && fallback.is_none() {
            let s = String::from_utf8_lossy(name.name).trim().to_string();
            if !s.is_empty() {
                fallback = Some(s);
            }
        }
    }
    fallback
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

    /// Inspects raw font bytes (enforcing TTF/glyf format & OS/2 fsType rules) without failing on
    /// Restricted/BitmapOnly so diagnostics callers can still inspect `report.embedding_permission`,
    /// OR validates strictly in [`Self::validate_and_inspect`].
    pub fn inspect_font_bytes(
        bytes: &[u8],
        fallback_family: &str,
    ) -> Result<(Self, FontInspectionReport), FontLoadError> {
        if bytes.len() < 12 {
            return Err(FontLoadError::InvalidFontBinary(
                "file smaller than 12-byte sfnt header".to_string(),
            ));
        }
        if &bytes[0..4] == b"wOFF" || &bytes[0..4] == b"wOF2" {
            return Err(FontLoadError::RejectWoff);
        }
        let face = ttf_parser::Face::parse(bytes, 0)
            .map_err(|e| FontLoadError::InvalidFontBinary(e.to_string()))?;
        if face.units_per_em() == 0 {
            return Err(FontLoadError::InvalidFontBinary(
                "unitsPerEm is zero".to_string(),
            ));
        }
        if face.tables().glyf.is_none() {
            return Err(FontLoadError::RejectCff);
        }

        let fs_type = face
            .raw_face()
            .table(ttf_parser::Tag::from_bytes(b"OS/2"))
            .and_then(|tbl| {
                if tbl.len() >= 10 {
                    Some(u16::from_be_bytes([tbl[8], tbl[9]]))
                } else {
                    None
                }
            })
            .unwrap_or(0);

        let embedding_permission = if (fs_type & 0x0200) != 0 {
            EmbeddingPermission::BitmapOnly
        } else if (fs_type & 0x0002) != 0 && (fs_type & 0x000C) == 0 {
            EmbeddingPermission::Restricted
        } else if (fs_type & 0x0008) != 0 {
            EmbeddingPermission::Editable
        } else if (fs_type & 0x0004) != 0 {
            EmbeddingPermission::PreviewAndPrint
        } else {
            EmbeddingPermission::Installable
        };
        let is_embeddable = !matches!(
            embedding_permission,
            EmbeddingPermission::Restricted | EmbeddingPermission::BitmapOnly
        );

        let parsed_family = extract_name_string(&face, ttf_parser::name_id::TYPOGRAPHIC_FAMILY)
            .or_else(|| extract_name_string(&face, ttf_parser::name_id::FAMILY))
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| fallback_family.trim().to_string());
        let final_family = if parsed_family.is_empty() {
            "CustomFont".to_string()
        } else {
            parsed_family
        };
        let postscript_name = extract_name_string(&face, ttf_parser::name_id::POST_SCRIPT_NAME);

        let weight = face.weight().to_number();
        let is_bold = weight >= 600 || face.is_bold();
        let is_italic = face.is_italic();

        let mut heb_hits = 0usize;
        for cp in 0x05D0u32..=0x05EAu32 {
            if let Some(ch) = char::from_u32(cp) {
                if face.glyph_index(ch).is_some() {
                    heb_hits += 1;
                }
            }
        }
        let niqqud_hits = NIQQUD_CODEPOINTS
            .iter()
            .filter(|&&ch| face.glyph_index(ch).is_some())
            .count();
        let mut taamim_hits = 0usize;
        let taamim_total = (0x05AFu32 - 0x0591u32 + 1) as usize;
        for cp in 0x0591u32..=0x05AFu32 {
            if let Some(ch) = char::from_u32(cp) {
                if face.glyph_index(ch).is_some() {
                    taamim_hits += 1;
                }
            }
        }

        let report = FontInspectionReport {
            family_name: final_family.clone(),
            postscript_name,
            weight,
            is_bold,
            is_italic,
            glyph_count: face.number_of_glyphs(),
            units_per_em: face.units_per_em(),
            fs_type,
            embedding_permission,
            is_embeddable,
            has_hebrew_letters: heb_hits == HEBREW_LETTERS_COUNT,
            hebrew_coverage_pct: (heb_hits as f32 / HEBREW_LETTERS_COUNT as f32) * 100.0,
            niqqud_coverage_pct: (niqqud_hits as f32 / NIQQUD_CODEPOINTS.len() as f32) * 100.0,
            taamim_coverage_pct: (taamim_hits as f32 / taamim_total as f32) * 100.0,
        };

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
        let font_data = Self {
            family_name: final_family,
            raw_bytes: Arc::new(bytes.to_vec()),
            metrics,
        };
        Ok((font_data, report))
    }

    /// Strictly validates external font bytes for TrueType outlines and embeddable license flags.
    pub fn validate_and_inspect(
        bytes: &[u8],
        fallback_family: &str,
    ) -> Result<(Self, FontInspectionReport), FontLoadError> {
        let (font_data, report) = Self::inspect_font_bytes(bytes, fallback_family)?;
        match report.embedding_permission {
            EmbeddingPermission::Restricted => Err(FontLoadError::RestrictedLicense),
            EmbeddingPermission::BitmapOnly => Err(FontLoadError::BitmapOnlyEmbedding),
            _ => Ok((font_data, report)),
        }
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
    manager: &'a FontManager,
}

impl<'a> ShapingSession<'a> {
    /// A session for another family from the same font manager, for runs whose span
    /// asks for a different font than the paragraph (or its bold face).
    pub fn sibling(&self, font_family: &str) -> ShapingSession<'a> {
        self.manager.session(font_family)
    }

    /// See [`FontManager::face_for`].
    pub fn face_for(&self, family: &str, weight: u16) -> String {
        self.manager.face_for(family, weight)
    }

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
    /// Families/faces loaded from external TTF files or `.tok` packages.
    external_keys: BTreeSet<String>,
    /// Cached inspection reports keyed by registered face name.
    inspection_reports: BTreeMap<String, FontInspectionReport>,
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
            external_keys: BTreeSet::new(),
            inspection_reports: BTreeMap::new(),
        }
    }

    pub fn register_font(&mut self, font: FontData) {
        self.fonts.insert(font.family_name.clone(), Arc::new(font));
    }

    /// Validates external font bytes and registers the font + weight entry on success.
    pub fn register_external_font_bytes(
        &mut self,
        bytes: Vec<u8>,
        fallback_family: &str,
    ) -> Result<FontInspectionReport, FontLoadError> {
        let (font_data, report) = FontData::validate_and_inspect(&bytes, fallback_family)?;
        let base_family = report.family_name.clone();
        let face_key = if report.is_bold && !base_family.ends_with(" Bold") {
            format!("{} Bold", base_family)
        } else {
            base_family.clone()
        };
        let mut stored = font_data;
        stored.family_name = face_key.clone();
        self.fonts.insert(face_key.clone(), Arc::new(stored));
        self.register_weight(&base_family, report.weight, &face_key);
        if !self.fonts.contains_key(&base_family) {
            if let Some(arc) = self.fonts.get(&face_key).cloned() {
                self.fonts.insert(base_family.clone(), arc);
            }
        }
        self.external_keys.insert(face_key.clone());
        self.inspection_reports.insert(face_key, report.clone());
        Ok(report)
    }

    /// Scans `dir` deterministically for `.ttf` / `.otf` files and registers valid external fonts.
    pub fn load_fonts_from_dir(
        &mut self,
        dir: &Path,
    ) -> Vec<(PathBuf, Result<FontInspectionReport, FontLoadError>)> {
        let mut results = Vec::new();
        let Ok(entries) = std::fs::read_dir(dir) else {
            return results;
        };
        let mut paths: Vec<PathBuf> = entries
            .filter_map(|e| e.ok().map(|ent| ent.path()))
            .filter(|p| {
                p.is_file()
                    && p.extension().and_then(|s| s.to_str()).is_some_and(|ext| {
                        ext.eq_ignore_ascii_case("ttf") || ext.eq_ignore_ascii_case("otf")
                    })
            })
            .collect();
        paths.sort();

        for path in paths {
            let fallback = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("CustomFont")
                .to_string();
            match std::fs::read(&path) {
                Ok(bytes) => {
                    let res = self.register_external_font_bytes(bytes, &fallback);
                    results.push((path, res));
                }
                Err(e) => {
                    results.push((path, Err(FontLoadError::InvalidFontBinary(e.to_string()))));
                }
            }
        }
        results
    }

    /// Registers installed, embeddable system TrueType fonts once per process and
    /// merges them into this manager without overriding embedded or user-installed fonts.
    pub fn load_system_fonts(&mut self) {
        type CachedSystemFont = (String, String, u16, Arc<FontData>, FontInspectionReport);
        static SYSTEM_FONTS_CACHE: std::sync::OnceLock<Vec<CachedSystemFont>> =
            std::sync::OnceLock::new();

        let cached = SYSTEM_FONTS_CACHE.get_or_init(|| {
            let mut dirs: Vec<PathBuf> = Vec::new();
            #[cfg(windows)]
            {
                if let Some(windir) = std::env::var_os("WINDIR") {
                    dirs.push(PathBuf::from(windir).join("Fonts"));
                } else {
                    dirs.push(PathBuf::from(r"C:\Windows\Fonts"));
                }
                if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
                    dirs.push(
                        PathBuf::from(local_app_data)
                            .join("Microsoft")
                            .join("Windows")
                            .join("Fonts"),
                    );
                }
            }
            #[cfg(target_os = "macos")]
            {
                dirs.push(PathBuf::from("/Library/Fonts"));
                dirs.push(PathBuf::from("/System/Library/Fonts"));
                if let Some(home) = std::env::var_os("HOME") {
                    dirs.push(PathBuf::from(home).join("Library/Fonts"));
                }
            }
            #[cfg(all(unix, not(target_os = "macos")))]
            {
                dirs.push(PathBuf::from("/usr/share/fonts"));
                dirs.push(PathBuf::from("/usr/local/share/fonts"));
                if let Some(home) = std::env::var_os("HOME") {
                    dirs.push(PathBuf::from(home).join(".local/share/fonts"));
                }
            }

            let mut out = Vec::new();
            for dir in dirs {
                let Ok(entries) = std::fs::read_dir(&dir) else {
                    continue;
                };
                let mut paths: Vec<PathBuf> = entries
                    .filter_map(|e| e.ok().map(|ent| ent.path()))
                    .filter(|p| {
                        p.is_file()
                            && p.extension()
                                .and_then(|s| s.to_str())
                                .is_some_and(|ext| ext.eq_ignore_ascii_case("ttf"))
                    })
                    .collect();
                paths.sort();

                for path in paths {
                    let Ok(bytes) = std::fs::read(&path) else {
                        continue;
                    };
                    let fallback = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("SystemFont");
                    if let Ok((mut font_data, report)) =
                        FontData::validate_and_inspect(&bytes, fallback)
                    {
                        let base_family = report.family_name.clone();
                        let face_key = if report.is_bold && !base_family.ends_with(BOLD_SUFFIX) {
                            format!("{base_family}{BOLD_SUFFIX}")
                        } else {
                            base_family.clone()
                        };
                        font_data.family_name = face_key.clone();
                        out.push((
                            base_family,
                            face_key,
                            report.weight,
                            Arc::new(font_data),
                            report,
                        ));
                    }
                }
            }
            out
        });

        for (base_family, face_key, weight, font_arc, report) in cached {
            if !self.fonts.contains_key(face_key) {
                self.fonts.insert(face_key.clone(), Arc::clone(font_arc));
                self.inspection_reports
                    .entry(face_key.clone())
                    .or_insert_with(|| report.clone());
            }
            self.register_weight(base_family, *weight, face_key);
            if !self.fonts.contains_key(base_family) {
                self.fonts.insert(base_family.clone(), Arc::clone(font_arc));
                self.inspection_reports
                    .entry(base_family.clone())
                    .or_insert_with(|| report.clone());
            }
        }
    }

    /// Returns all externally registered fonts `(face_name, FontData)` not embedded by default.
    pub fn external_fonts(&self) -> Vec<(String, Arc<FontData>)> {
        self.external_keys
            .iter()
            .filter_map(|k| self.fonts.get(k).map(|f| (k.clone(), Arc::clone(f))))
            .collect()
    }

    /// Returns true if `family` (exact, alias, or case-insensitive) exists in the manager without falling back blindly.
    pub fn is_family_available(&self, family: &str) -> bool {
        self.font_exact(family).is_some()
    }

    /// Returns inspection report for `family` (computing lazily from loaded font bytes if needed).
    pub fn inspect_registered_family(&self, family: &str) -> Option<FontInspectionReport> {
        if let Some(r) = self.inspection_reports.get(family) {
            return Some(r.clone());
        }
        self.font_exact(family)
            .and_then(|f| FontData::inspect_font_bytes(&f.raw_bytes, &f.family_name).ok())
            .map(|(_, r)| r)
    }

    /// Scans `doc` paragraphs, styles, and character formats to report missing families
    /// or fonts missing Hebrew/Niqqud/Taamim glyphs actually needed by the document text.
    pub fn find_missing_or_incomplete_fonts(
        &self,
        doc: &tok_core::model::DocumentRoot,
    ) -> Vec<MissingFontDiagnostic> {
        struct Usage {
            paras: usize,
            needs_hebrew: bool,
            needs_niqqud: bool,
            needs_taamim: bool,
        }
        let mut usage_by_family: BTreeMap<String, Usage> = BTreeMap::new();
        for s in &doc.paragraph_styles {
            if !s.font_family.trim().is_empty() {
                usage_by_family
                    .entry(s.font_family.clone())
                    .or_insert(Usage {
                        paras: 0,
                        needs_hebrew: false,
                        needs_niqqud: false,
                        needs_taamim: false,
                    });
            }
        }
        for cs in &doc.character_styles {
            if let Some(fam) = &cs.font_family {
                if !fam.trim().is_empty() {
                    usage_by_family.entry(fam.clone()).or_insert(Usage {
                        paras: 0,
                        needs_hebrew: false,
                        needs_niqqud: false,
                        needs_taamim: false,
                    });
                }
            }
        }

        for sec in &doc.sections {
            for flow in &sec.flows {
                for p in &flow.paragraphs {
                    let eff = p.effective_style(&doc.paragraph_styles);
                    let has_heb = p
                        .text
                        .chars()
                        .any(|c| ('\u{05D0}'..='\u{05EA}').contains(&c));
                    let has_niq = p.text.chars().any(|c| NIQQUD_CODEPOINTS.contains(&c));
                    let has_taa = p
                        .text
                        .chars()
                        .any(|c| ('\u{0591}'..='\u{05AF}').contains(&c));

                    if !eff.font_family.trim().is_empty() {
                        let u = usage_by_family
                            .entry(eff.font_family.clone())
                            .or_insert(Usage {
                                paras: 0,
                                needs_hebrew: false,
                                needs_niqqud: false,
                                needs_taamim: false,
                            });
                        u.paras += 1;
                        u.needs_hebrew |= has_heb;
                        u.needs_niqqud |= has_niq;
                        u.needs_taamim |= has_taa;
                    }
                    for sp in &p.spans {
                        if let Some(fam) = &sp.format.font_family {
                            if !fam.trim().is_empty() {
                                let slice: String = p
                                    .text
                                    .chars()
                                    .skip(sp.start)
                                    .take(sp.end.saturating_sub(sp.start))
                                    .collect();
                                let u = usage_by_family.entry(fam.clone()).or_insert(Usage {
                                    paras: 0,
                                    needs_hebrew: false,
                                    needs_niqqud: false,
                                    needs_taamim: false,
                                });
                                u.paras += 1;
                                u.needs_hebrew |= slice
                                    .chars()
                                    .any(|c| ('\u{05D0}'..='\u{05EA}').contains(&c));
                                u.needs_niqqud |=
                                    slice.chars().any(|c| NIQQUD_CODEPOINTS.contains(&c));
                                u.needs_taamim |= slice
                                    .chars()
                                    .any(|c| ('\u{0591}'..='\u{05AF}').contains(&c));
                            }
                        }
                    }
                }
            }
        }

        let mut out = Vec::new();
        for (fam, u) in usage_by_family {
            if !self.is_family_available(&fam) {
                out.push(MissingFontDiagnostic {
                    family_name: fam,
                    missing_family: true,
                    missing_hebrew_coverage: u.needs_hebrew,
                    missing_niqqud_coverage: u.needs_niqqud,
                    missing_taamim_coverage: u.needs_taamim,
                    affected_paragraphs: u.paras,
                });
                continue;
            }
            if let Some(rep) = self.inspect_registered_family(&fam) {
                let missing_heb = u.needs_hebrew && !rep.has_hebrew_letters;
                let missing_niq = u.needs_niqqud && rep.niqqud_coverage_pct < 50.0;
                let missing_taa = u.needs_taamim && rep.taamim_coverage_pct < 50.0;
                if missing_heb || missing_niq || missing_taa {
                    out.push(MissingFontDiagnostic {
                        family_name: fam,
                        missing_family: false,
                        missing_hebrew_coverage: missing_heb,
                        missing_niqqud_coverage: missing_niq,
                        missing_taamim_coverage: missing_taa,
                        affected_paragraphs: u.paras,
                    });
                }
            }
        }
        out
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
        self.font_exact(family).or_else(|| {
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

    /// Exact lookup of a registered family or alias (with case-insensitive fallback), without default fallback.
    pub fn font_exact(&self, family: &str) -> Option<&Arc<FontData>> {
        self.fonts.get(family).or_else(|| {
            let trimmed = family.trim();
            self.fonts
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(trimmed))
                .map(|(_, v)| v)
        })
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
        ShapingSession {
            primary,
            fallbacks,
            manager: self,
        }
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

    fn patch_sfnt_table<F>(base_ttf: &[u8], tag: &[u8; 4], mut f: F) -> Vec<u8>
    where
        F: FnMut(&mut [u8], usize, usize),
    {
        let mut bytes = base_ttf.to_vec();
        let num_tables = u16::from_be_bytes([bytes[4], bytes[5]]) as usize;
        for i in 0..num_tables {
            let rec = 12 + i * 16;
            if &bytes[rec..rec + 4] == tag {
                let offset = u32::from_be_bytes([
                    bytes[rec + 8],
                    bytes[rec + 9],
                    bytes[rec + 10],
                    bytes[rec + 11],
                ]) as usize;
                let len = u32::from_be_bytes([
                    bytes[rec + 12],
                    bytes[rec + 13],
                    bytes[rec + 14],
                    bytes[rec + 15],
                ]) as usize;
                f(&mut bytes, rec, offset);
                let _ = len;
                break;
            }
        }
        bytes
    }

    #[test]
    fn validates_fstype_woff_cff_and_hebrew_coverage() {
        // 1. WOFF / WOFF2 rejected
        assert_eq!(
            FontData::validate_and_inspect(b"wOFF\x00\x01\x00\x00\x00\x00\x00\x20extra", "Bad")
                .unwrap_err(),
            FontLoadError::RejectWoff
        );
        assert_eq!(
            FontData::validate_and_inspect(b"wOF2\x00\x01\x00\x00\x00\x00\x00\x20extra", "Bad")
                .unwrap_err(),
            FontLoadError::RejectWoff
        );

        // 2. Embedded Noto Serif Hebrew is Installable and has full Hebrew + Niqqud + Taamim
        let (font, report) =
            FontData::validate_and_inspect(EMBEDDED_NOTO_SERIF_HEBREW, "Fallback").unwrap();
        assert_eq!(font.family_name, "Noto Serif Hebrew");
        assert!(report.is_embeddable);
        assert!(report.has_hebrew_letters);
        assert!(report.hebrew_coverage_pct > 99.0);
        assert!(report.niqqud_coverage_pct > 90.0);
        assert!(report.taamim_coverage_pct > 80.0);

        // 3. CFF-only font (no glyf table) rejected
        let cff_like = patch_sfnt_table(EMBEDDED_NOTO_SERIF_HEBREW, b"glyf", |buf, rec, _| {
            buf[rec..rec + 4].copy_from_slice(b"CFF ");
        });
        assert_eq!(
            FontData::validate_and_inspect(&cff_like, "CFF").unwrap_err(),
            FontLoadError::RejectCff
        );

        // 4. OS/2 fsType enforcement: 0x0002 Restricted -> Err, 0x0200 BitmapOnly -> Err, 0x0004/0x0008 -> Ok
        let restricted = patch_sfnt_table(EMBEDDED_NOTO_SERIF_HEBREW, b"OS/2", |buf, _, off| {
            buf[off + 8..off + 10].copy_from_slice(&0x0002u16.to_be_bytes());
        });
        assert_eq!(
            FontData::validate_and_inspect(&restricted, "Restricted").unwrap_err(),
            FontLoadError::RestrictedLicense
        );

        let bitmap_only = patch_sfnt_table(EMBEDDED_NOTO_SERIF_HEBREW, b"OS/2", |buf, _, off| {
            buf[off + 8..off + 10].copy_from_slice(&0x0200u16.to_be_bytes());
        });
        assert_eq!(
            FontData::validate_and_inspect(&bitmap_only, "Bitmap").unwrap_err(),
            FontLoadError::BitmapOnlyEmbedding
        );

        let editable = patch_sfnt_table(EMBEDDED_NOTO_SERIF_HEBREW, b"OS/2", |buf, _, off| {
            buf[off + 8..off + 10].copy_from_slice(&0x0008u16.to_be_bytes());
        });
        let (_, rep_ed) = FontData::validate_and_inspect(&editable, "Ed").unwrap();
        assert_eq!(rep_ed.embedding_permission, EmbeddingPermission::Editable);
    }

    #[test]
    fn finds_missing_or_incomplete_fonts_in_document() {
        use tok_core::id::FractionalIndex;
        use tok_core::model::{DocumentRoot, ParagraphNode};
        use tok_core::styles::ParagraphStyle;

        let mgr = FontManager::default();
        let mut doc = DocumentRoot::new("Missing Fonts Test");
        doc.paragraph_styles.push(ParagraphStyle {
            id: "custom-missing".to_string(),
            name: "Custom Missing".to_string(),
            font_family: "NonExistentDavid".to_string(),
            ..Default::default()
        });
        doc.sections[0].flows[0].paragraphs = vec![ParagraphNode::new(
            FractionalIndex::initial(),
            "custom-missing",
            "שלום עולם",
        )];

        let diags = mgr.find_missing_or_incomplete_fonts(&doc);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].family_name, "NonExistentDavid");
        assert!(diags[0].missing_family);
        assert!(diags[0].missing_hebrew_coverage);
        assert_eq!(diags[0].affected_paragraphs, 1);
    }
}
