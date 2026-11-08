//! Text Shaping and Font Metrics Engine via rustybuzz (HarfBuzz-compatible).
//!
//! Enforces:
//! - Monotone grapheme clusters: `BufferClusterLevel::MonotoneGraphemes`
//! - OpenType GPOS/GSUB features: `mark`, `mkmk`, `kern`, `liga`, `ccmp`
//! - Full support for TrueType and OpenType fonts

use rustybuzz::ttf_parser::Tag;
use rustybuzz::{BufferClusterLevel, Direction, Face, Feature, UnicodeBuffer};

#[derive(Debug, Clone, PartialEq)]
pub struct PositionedGlyph {
    pub glyph_id: u32,
    pub cluster: u32,
    pub x_advance: f32, // in points (pt)
    pub y_advance: f32,
    pub x_offset: f32,
    pub y_offset: f32,
    pub character: Option<char>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShapedRun {
    pub glyphs: Vec<PositionedGlyph>,
    pub total_width_pt: f32,
    pub font_size_pt: f32,
    pub is_rtl: bool,
}

fn cluster_character(text: &str, byte_offset: u32) -> Option<char> {
    text.get(byte_offset as usize..)
        .and_then(|s| s.chars().next())
}

pub struct TextShaper;

impl TextShaper {
    /// Shapes text using an OpenType/TrueType font face with default features.
    pub fn shape_with_face(face: &Face, text: &str, font_size_pt: f32, is_rtl: bool) -> ShapedRun {
        Self::shape_with_face_and_features(face, text, font_size_pt, is_rtl, &[])
    }

    /// Shapes text using an OpenType/TrueType font face with additional custom features (e.g. ss01, ss04, calt).
    pub fn shape_with_face_and_features(
        face: &Face,
        text: &str,
        font_size_pt: f32,
        is_rtl: bool,
        extra_features: &[Feature],
    ) -> ShapedRun {
        let mut buffer = UnicodeBuffer::new();
        buffer.set_cluster_level(BufferClusterLevel::MonotoneGraphemes);
        buffer.set_direction(if is_rtl {
            Direction::RightToLeft
        } else {
            Direction::LeftToRight
        });
        buffer.push_str(text);

        // Standard OpenType typography features (always including contextual alternates)
        let mut features = vec![
            Feature::new(Tag::from_bytes(b"kern"), 1, ..),
            Feature::new(Tag::from_bytes(b"liga"), 1, ..),
            Feature::new(Tag::from_bytes(b"mark"), 1, ..),
            Feature::new(Tag::from_bytes(b"mkmk"), 1, ..),
            Feature::new(Tag::from_bytes(b"ccmp"), 1, ..),
            Feature::new(Tag::from_bytes(b"calt"), 1, ..),
        ];

        for feat in extra_features {
            if let Some(pos) = features.iter().position(|f| f.tag == feat.tag) {
                features[pos] = *feat;
            } else {
                features.push(*feat);
            }
        }

        let glyph_buffer = rustybuzz::shape(face, &features, buffer);
        let infos = glyph_buffer.glyph_infos();
        let positions = glyph_buffer.glyph_positions();

        let upem = face.units_per_em();
        let scale = if upem <= 0 {
            0.0
        } else {
            font_size_pt / upem as f32
        };

        // Cluster boundaries, to find the source text of each cluster.
        let mut boundaries: Vec<u32> = infos.iter().map(|i| i.cluster).collect();
        boundaries.sort_unstable();
        boundaries.dedup();

        let mut glyphs = Vec::with_capacity(infos.len());
        let mut total_width_pt = 0.0;

        for (info, pos) in infos.iter().zip(positions.iter()) {
            let mut x_adv = pos.x_advance as f32 * scale;
            let y_adv = pos.y_advance as f32 * scale;
            let x_off = pos.x_offset as f32 * scale;
            let y_off = pos.y_offset as f32 * scale;

            // rustybuzz clusters are UTF-8 byte offsets, not Unicode scalar indices.
            // A grapheme cluster holds a base letter and its marks; attribute each
            // glyph to the character it encodes (so niqqud glyphs report the mark,
            // not their base letter), falling back to the cluster's first character
            // for ligatures and other substituted glyphs.
            let start = info.cluster as usize;
            let end = boundaries
                .get(boundaries.partition_point(|&b| b <= info.cluster))
                .map_or(text.len(), |&b| b as usize);
            let character = text
                .get(start..end)
                .and_then(|cluster_text| {
                    cluster_text.chars().find(|&c| {
                        face.glyph_index(c).map(|g| u32::from(g.0)) == Some(info.glyph_id)
                    })
                })
                .or_else(|| cluster_character(text, info.cluster));

            // Ensure Masoretic punctuation such as Inverted Nun (U+05C6) never collapses
            // to zero advance width even if a font face lacks an advance for it.
            if character == Some('\u{05C6}') && x_adv <= 0.0 {
                x_adv = font_size_pt * 0.55;
            }

            total_width_pt += x_adv;

            glyphs.push(PositionedGlyph {
                glyph_id: info.glyph_id,
                cluster: info.cluster,
                x_advance: x_adv,
                y_advance: y_adv,
                x_offset: x_off,
                y_offset: y_off,
                character,
            });
        }

        ShapedRun {
            glyphs,
            total_width_pt,
            font_size_pt,
            is_rtl,
        }
    }

    /// Built-in fallback shaper based on standard typographic metric proportions.
    /// Used for headless testing and when external font files are loading.
    pub fn shape_fallback(text: &str, font_size_pt: f32, is_rtl: bool) -> ShapedRun {
        let mut glyphs = Vec::new();
        let mut total_width_pt = 0.0;

        for (i, ch) in text.char_indices() {
            // Niqqud, Dagesh, Te'amim have 0 advance width in fallback
            let width_ratio = match ch as u32 {
                0x05C6 => 0.55,          // Inverted Nun (נו"ן הפוכה U+05C6) - distinct letter glyph
                0x05BE => 0.40,          // Maqaf (מקף)
                0x05C0 => 0.25,          // Paseq (פסק)
                0x05C3 => 0.40,          // Sof Pasuq (סוף פסוק)
                0x0591..=0x05C7 => 0.0,  // Combining marks: zero width
                0x0020 => 0.28,          // Space
                0x05D0..=0x05EA => 0.55, // Standard Hebrew letter
                0x0061..=0x007A => 0.45, // 'a'..='z'
                0x0041..=0x005A => 0.60, // 'A'..='Z'
                0x0030..=0x0039 => 0.50, // '0'..='9'
                _ => 0.50,
            };

            let x_adv = font_size_pt * width_ratio;
            total_width_pt += x_adv;

            glyphs.push(PositionedGlyph {
                glyph_id: ch as u32,
                cluster: i as u32,
                x_advance: x_adv,
                y_advance: 0.0,
                x_offset: 0.0,
                y_offset: 0.0,
                character: Some(ch),
            });
        }

        ShapedRun {
            glyphs,
            total_width_pt,
            font_size_pt,
            is_rtl,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clusters_are_utf8_byte_offsets() {
        let text = "aבּ😀ג";
        let run = TextShaper::shape_fallback(text, 12.0, true);
        for ((offset, ch), glyph) in text.char_indices().zip(&run.glyphs) {
            assert_eq!(glyph.cluster, offset as u32);
            assert_eq!(cluster_character(text, glyph.cluster), Some(ch));
        }
        assert_eq!(cluster_character(text, 2), None);
        assert_eq!(cluster_character(text, text.len() as u32), None);
    }

    /// Regression: every glyph of a grapheme cluster reported the base letter,
    /// so PDF ToUnicode mapped niqqud glyphs to duplicate letters.
    #[test]
    fn mark_glyphs_report_their_own_character() {
        let mgr = crate::font::FontManager::default();
        let font = mgr.get_font("Noto Serif Hebrew").unwrap();
        let face = font.shaping_face().unwrap();
        let run = TextShaper::shape_with_face(&face, "אָלֶף", 12.0, true);
        let chars: Vec<char> = run.glyphs.iter().filter_map(|g| g.character).collect();
        for expected in ['א', '\u{05B8}', 'ל', '\u{05B6}', 'ף'] {
            assert_eq!(
                chars.iter().filter(|&&c| c == expected).count(),
                1,
                "{expected:?} in {chars:?}"
            );
        }
        // Clusters stay grapheme-based: the mark shares its letter's cluster.
        let qamats = run
            .glyphs
            .iter()
            .find(|g| g.character == Some('\u{05B8}'))
            .unwrap();
        assert_eq!(qamats.cluster, 0);
    }

    #[test]
    fn test_shape_fallback_hebrew() {
        let text = "בְּרֵאשִׁית";
        let run = TextShaper::shape_fallback(text, 12.0, true);
        assert!(run.is_rtl);
        // Combining marks should have 0 advance
        let non_zero_advances = run.glyphs.iter().filter(|g| g.x_advance > 0.0).count();
        // 6 base Hebrew letters: ב, ר, א, ש, י, ת
        assert_eq!(non_zero_advances, 6);
        assert!(run.total_width_pt > 0.0);
    }

    #[test]
    fn test_shape_fallback_nun_hafukha_and_masoretic_punctuation() {
        let text = "\u{05C6}\u{05BE}\u{05C0}\u{05C3}";
        let run = TextShaper::shape_fallback(text, 12.0, true);
        assert_eq!(run.glyphs.len(), 4);
        for g in &run.glyphs {
            assert!(
                g.x_advance > 0.0,
                "Masoretic punctuation must have non-zero advance width"
            );
        }
    }
}
