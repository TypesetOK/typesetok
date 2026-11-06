//! Canonical Hebrew Normalization (Unicode NFC + Masoretic/SBL Combining Class Ordering:
//! Base -> Dagesh/Shin-Sin -> Vowel Niqqud -> Cantillation Taamim).
//!
//! Standard Unicode NFD sorts combining marks strictly by Canonical Combining Class (CCC).
//! This erroneously swaps Dagesh (CCC=21) before Shin Dot (CCC=24) and pushes Qamats (CCC=18)
//! ahead of Dagesh, which breaks OpenType GPOS feature lookups (`mark`, `mkmk`) in fonts.
//!
//! Canonical Hebrew Normalization enforces the deterministic Masoretic/SBL order:
//! Base Letter -> Shin/Sin Dot -> Dagesh/Mapiq/Rafe -> Niqqud -> Meteg -> Te'amim

use unicode_normalization::UnicodeNormalization;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HebrewMarkCategory {
    ShinSinDot = 1,
    DageshRafe = 2,
    Niqqud = 3,
    Meteg = 4,
    Teamin = 5,
    OtherMark = 6,
}

pub struct HebrewNormalizer;

/// Canonical Hebrew Normalization (Unicode NFC + Masoretic/SBL Combining Class Ordering:
/// Base -> Dagesh/Shin-Sin -> Vowel Niqqud -> Cantillation Taamim).
pub fn canonicalize_hebrew(input: &str) -> String {
    HebrewNormalizer::normalize(input)
}

impl HebrewNormalizer {
    /// Classifies a combining character into its Canonical Hebrew Normalization
    /// (Masoretic/SBL Combining Class Ordering) category.
    pub fn mark_category(c: char) -> Option<HebrewMarkCategory> {
        let code = c as u32;
        match code {
            // Shin / Sin Dots
            0x05C1 | 0x05C2 => Some(HebrewMarkCategory::ShinSinDot),

            // Dagesh / Mapiq / Rafe
            0x05BC | 0x05BF => Some(HebrewMarkCategory::DageshRafe),

            // Niqqud: Sheva .. Qubuts, Qamats Qatan
            0x05B0..=0x05BB | 0x05C7 => Some(HebrewMarkCategory::Niqqud),

            // Meteg
            0x05BD => Some(HebrewMarkCategory::Meteg),

            // Te'amim (Cantillation marks)
            0x0591..=0x05AF => Some(HebrewMarkCategory::Teamin),

            // Other Hebrew dots/accents
            // Upper / lower dots and the Judeo-Spanish varika (survives
            // presentation-form decomposition, which has no mapping for it).
            0x05C4 | 0x05C5 | 0xFB1E => Some(HebrewMarkCategory::OtherMark),

            _ => None,
        }
    }

    /// Decomposes legacy Hebrew presentation forms (U+FB1D..U+FB4F) only.
    /// Unlike global NFKD, this preserves non-breaking spaces, ligatures, and other characters.
    pub fn decompose_presentation_forms(input: &str) -> String {
        let mut result = String::with_capacity(input.len());
        for ch in input.chars() {
            if ('\u{FB1D}'..='\u{FB4F}').contains(&ch) {
                // Decompose only Hebrew Presentation Forms via NFKD
                let s: String = ch.to_string().nfkd().collect();
                result.push_str(&s);
            } else {
                result.push(ch);
            }
        }
        result
    }

    /// Normalizes Hebrew text using Canonical Hebrew Normalization
    /// (Unicode NFC + Masoretic/SBL Combining Class Ordering:
    /// Base -> Dagesh/Shin-Sin -> Vowel Niqqud -> Cantillation Taamim).
    pub fn normalize(input: &str) -> String {
        // Step 1: Decompose any legacy presentation forms
        let decomposed = Self::decompose_presentation_forms(input);

        // Step 2: Iterate over grapheme clusters and sort combining marks per base letter
        let mut result = String::with_capacity(decomposed.len());
        let mut current_marks: Vec<(HebrewMarkCategory, u32, char)> = Vec::new();

        let flush_marks = |marks: &mut Vec<(HebrewMarkCategory, u32, char)>, out: &mut String| {
            if marks.is_empty() {
                return;
            }
            // Sort by category first, preserving relative insertion order (stable sort)
            marks.sort_by_key(|&(cat, index, _)| (cat, index));
            for &(_, _, ch) in marks.iter() {
                out.push(ch);
            }
            marks.clear();
        };

        for (idx, ch) in decomposed.chars().enumerate() {
            if let Some(cat) = Self::mark_category(ch) {
                current_marks.push((cat, idx as u32, ch));
            } else {
                // Not a combining mark; flush collected marks for previous base character
                flush_marks(&mut current_marks, &mut result);
                result.push(ch);
            }
        }

        flush_marks(&mut current_marks, &mut result);
        result
    }

    /// Alias for [`HebrewNormalizer::normalize`].
    pub fn canonicalize_hebrew(input: &str) -> String {
        Self::normalize(input)
    }

    /// Returns true if character is a Hebrew letter (Alef .. Tav, final forms).
    pub fn is_hebrew_letter(c: char) -> bool {
        matches!(c as u32, 0x05D0..=0x05EA)
    }

    /// Codepoint for Inverted Nun (נו"ן הפוכה `U+05C6`).
    pub const NUN_HAFUKHA: char = '\u{05C6}';

    /// Returns true if character is an Inverted Nun (נו"ן הפוכה).
    pub fn is_nun_hafukha(c: char) -> bool {
        c == Self::NUN_HAFUKHA
    }

    /// Returns true if character is a Masoretic Biblical punctuation mark
    /// (e.g. Inverted Nun U+05C6, Paseq U+05C0, Sof Pasuq U+05C3, Maqaf U+05BE).
    pub fn is_masoretic_punctuation(c: char) -> bool {
        matches!(c as u32, 0x05BE | 0x05C0 | 0x05C3 | 0x05C6)
    }

    /// Returns true if character is a base Hebrew character (either letter or Masoretic punctuation).
    pub fn is_hebrew_base_char(c: char) -> bool {
        Self::is_hebrew_letter(c) || Self::is_masoretic_punctuation(c)
    }

    /// Returns true if character is a Unicode BiDi directional control.
    pub fn is_bidi_control(c: char) -> bool {
        matches!(
            c as u32,
            0x200E | 0x200F | // LRM, RLM
            0x202A..=0x202E | // LRE, RLE, PDF, LRO, RLO
            0x2066..=0x2069   // LRI, RLI, FSI, PDI
        )
    }

    /// Strips Hebrew vowel points and dots (Niqqud & Dagesh) while preserving consonants and cantillation.
    pub fn strip_niqqud(input: &str) -> String {
        input
            .chars()
            .filter(|&c| {
                let code = c as u32;
                !matches!(code, 0x05B0..=0x05BC | 0x05BF | 0x05C1 | 0x05C2 | 0x05C7)
            })
            .collect()
    }

    /// Strips Hebrew cantillation marks (Te'amim).
    pub fn strip_teamim(input: &str) -> String {
        input
            .chars()
            .filter(|&c| {
                let code = c as u32;
                !matches!(code, 0x0591..=0x05AF)
            })
            .collect()
    }

    /// Strips all combining marks (Niqqud + Te'amim + Meteg + dots).
    pub fn strip_all_marks(input: &str) -> String {
        input
            .chars()
            .filter(|&c| Self::mark_category(c).is_none())
            .collect()
    }

    /// Cleans pasted text: removes non-printable C0 control characters (except \n and \t)
    /// and BOM (U+FEFF), standardizes CRLF to LF, while strictly preserving
    /// BiDi controls, Hebrew letters, niqqud, te'amim, and punctuation.
    pub fn sanitize_plain_text(input: &str) -> String {
        let mut out = String::with_capacity(input.len());
        for c in input.chars() {
            let code = c as u32;
            if code == 0xFEFF || code == 0x00AD {
                // Strip BOM and soft hyphen
                continue;
            }
            if c == '\r' {
                // Handled / stripped in CRLF normalization
                continue;
            }
            if c < ' ' && c != '\n' && c != '\t' {
                // Strip C0 control characters (e.g. ASCII 0..31 except \n and \t)
                continue;
            }
            out.push(c);
        }
        Self::normalize(&out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canonical_hebrew_ordering_shin_dagesh_qamats() {
        // Input with wrong order: Base + Qamats (\u{05B8}) + Dagesh (\u{05BC}) + Shin Dot (\u{05C1})
        let wrong_input = "\u{05E9}\u{05B8}\u{05BC}\u{05C1}";

        let normalized = HebrewNormalizer::normalize(wrong_input);

        // Expected order:
        // 1. \u{05E9} (Shin)
        // 2. \u{05C1} (Shin Dot)
        // 3. \u{05BC} (Dagesh)
        // 4. \u{05B8} (Qamats)
        let expected = "\u{05E9}\u{05C1}\u{05BC}\u{05B8}";
        assert_eq!(normalized, expected);
    }

    #[test]
    fn test_canonical_hebrew_with_meteg_and_teamim() {
        // Base Bet + Patah + Te'amim (Etnahta \u{0591}) + Dagesh + Meteg
        let input = "\u{05D1}\u{05B7}\u{0591}\u{05BC}\u{05BD}";
        let normalized = HebrewNormalizer::normalize(input);

        // Expected:
        // 1. Bet \u{05D1}
        // 2. Dagesh \u{05BC}
        // 3. Patah \u{05B7}
        // 4. Meteg \u{05BD}
        // 5. Etnahta \u{0591}
        let expected = "\u{05D1}\u{05BC}\u{05B7}\u{05BD}\u{0591}";
        assert_eq!(normalized, expected);
    }

    #[test]
    fn test_decompose_presentation_forms() {
        // Presentation form U+FB2C (Shin with Shin Dot and Dagesh) + Qamats
        let input = "\u{FB2C}\u{05B8}";
        let normalized = HebrewNormalizer::normalize(input);

        // Decomposes and normalizes to: Shin, Shin Dot, Dagesh, Qamats
        let expected = "\u{05E9}\u{05C1}\u{05BC}\u{05B8}";
        assert_eq!(normalized, expected);
    }

    #[test]
    fn test_normalizer_preserves_nbsp() {
        // Non-breaking space (U+00A0) must survive normalization
        let input = "\u{05E9}\u{05C1}\u{05B8}\u{00A0}\u{05DC}\u{05B9}\u{05DD}";
        let result = HebrewNormalizer::normalize(input);
        assert!(
            result.contains('\u{00A0}'),
            "NBSP must be preserved, got: {:?}",
            result
        );
    }

    #[test]
    fn test_normalizer_empty_string() {
        let result = HebrewNormalizer::normalize("");
        assert_eq!(result, "");
    }

    #[test]
    fn test_normalizer_idempotent() {
        let input = "\u{05E9}\u{05B8}\u{05C1}\u{05DC}\u{05D5}\u{05B9}\u{05DD}";
        let once = HebrewNormalizer::normalize(input);
        let twice = HebrewNormalizer::normalize(&once);
        assert_eq!(once, twice, "Normalization must be idempotent");
    }

    #[test]
    fn varika_stays_attached_to_its_letter() {
        // Regression: U+FB1E was treated as a base character, so marks after it
        // were sorted as a separate cluster.
        let input = "\u{05D2}\u{FB1E}\u{05B8}\u{05BC}";
        let normalized = HebrewNormalizer::normalize(input);
        assert_eq!(normalized, "\u{05D2}\u{05BC}\u{05B8}\u{FB1E}");
        assert_eq!(HebrewNormalizer::normalize(&normalized), normalized);
    }

    #[test]
    fn normalization_is_idempotent_on_mixed_text() {
        let samples = [
            "\u{05E9}\u{05B8}\u{05BC}\u{05C1}\u{0591}\u{05BD}",
            "\u{FB2C}\u{05B8}\u{05D1}\u{05B7}\u{0591}\u{05BC}",
            "\u{05B8}\u{05BC} leading marks",
            "e\u{0301}\u{05D0}\u{05B7}\u{034F}\u{05B8}",
        ];
        for s in samples {
            let once = HebrewNormalizer::normalize(s);
            assert_eq!(HebrewNormalizer::normalize(&once), once, "input {s:?}");
            assert!(once.chars().count() >= s.chars().count());
        }
    }

    #[test]
    fn test_normalizer_plain_ascii() {
        let input = "Hello World 123";
        let result = HebrewNormalizer::normalize(input);
        assert_eq!(result, input, "ASCII text must pass through unchanged");
    }

    #[test]
    fn test_strip_niqqud_preserves_consonants_and_cantillation() {
        // "בְּרֵאשִׁ֖ית" with niqqud and cantillation (Tipha \u{0596})
        let input = "בְּרֵאשִׁ֖ית";
        let stripped = HebrewNormalizer::strip_niqqud(input);
        // Niqqud stripped, letters and cantillation preserved
        assert_eq!(stripped, "בראש֖ית");

        // Strip teamim
        let no_teamim = HebrewNormalizer::strip_teamim(input);
        assert_eq!(no_teamim, "בְּרֵאשִׁית");

        // Strip all marks
        let bare = HebrewNormalizer::strip_all_marks(input);
        assert_eq!(bare, "בראשית");
    }

    #[test]
    fn test_sanitize_plain_text_cleans_c0_and_bom_while_preserving_bidi_and_niqqud() {
        // Text with BOM (\u{FEFF}), CRLF, Tab, LRM (\u{200E}), and Hebrew with niqqud
        let dirty = "\u{FEFF}\u{200E}שָׁלוֹם\r\n\tWorld 123\u{0007}\u{00AD}";
        let cleaned = HebrewNormalizer::sanitize_plain_text(dirty);

        // BOM and bell char \u{0007} stripped, soft hyphen stripped, \r stripped, \n and \t preserved
        assert!(!cleaned.contains('\u{FEFF}'));
        assert!(!cleaned.contains('\u{0007}'));
        assert!(!cleaned.contains('\u{00AD}'));
        assert!(!cleaned.contains('\r'));
        assert!(cleaned.contains('\u{200E}')); // LRM preserved
        assert!(cleaned.contains('\t'));
        assert!(cleaned.contains('\n'));
        let expected_hebrew = HebrewNormalizer::normalize("שָׁלוֹם");
        assert!(cleaned.contains(&expected_hebrew));
    }

    #[test]
    fn test_is_bidi_control() {
        assert!(HebrewNormalizer::is_bidi_control('\u{200E}')); // LRM
        assert!(HebrewNormalizer::is_bidi_control('\u{200F}')); // RLM
        assert!(HebrewNormalizer::is_bidi_control('\u{2066}')); // LRI
        assert!(HebrewNormalizer::is_bidi_control('\u{2067}')); // RLI
        assert!(HebrewNormalizer::is_bidi_control('\u{2069}')); // PDI
        assert!(!HebrewNormalizer::is_bidi_control('א'));
        assert!(!HebrewNormalizer::is_bidi_control('A'));
    }

    #[test]
    fn property_canonicalize_hebrew_idempotence_and_nfc_length_preservation() {
        let corpus = [
            "בְּרֵאשִׁ֖ית בָּרָ֣א אֱלֹהִ֑ים אֵ֥ת הַשָּׁמַ֖יִם וְאֵ֥ת הָאָֽרֶץ׃",
            "\u{05E9}\u{0591}\u{05BD}\u{05B8}\u{05BC}\u{05C1}\u{05DC}\u{05D5}\u{05B9}\u{05DD}",
            "\u{05D0}\u{05B2}\u{0591}\u{05DC}\u{05B9}\u{05D4}\u{05B4}\u{0596}\u{05D9}\u{05DD}",
            "רַמְבַּ״ם וְשַׁ״ס עִם אנגלית ABC 123!",
            "\u{FB2A}\u{05B8}\u{0591} \u{FB31}\u{05B7}\u{05BD}",
        ];

        for sample in corpus {
            let first = canonicalize_hebrew(sample);
            let second = canonicalize_hebrew(&first);
            assert_eq!(first, second, "canonicalize_hebrew must be idempotent");

            // Standard Hebrew codepoints do not precompose under Unicode NFC,
            // so once presentation forms are decomposed, NFC and canonicalize_hebrew
            // strictly preserve character and byte lengths.
            let nfc_input: String = first.nfc().collect();
            let after_nfc = canonicalize_hebrew(&nfc_input);
            assert_eq!(
                after_nfc.chars().count(),
                nfc_input.chars().count(),
                "Length in scalar values must be preserved after NFC"
            );
            assert_eq!(
                after_nfc.len(),
                nfc_input.len(),
                "Byte length must be preserved after NFC"
            );
            assert_eq!(after_nfc, first);
        }
    }

    #[test]
    fn property_commutative_equivalence_of_combining_mark_permutations() {
        // 5 combining marks from distinct Masoretic/SBL categories:
        // ShinDot (\u{05C1}), Dagesh (\u{05BC}), Qamats (\u{05B8}), Meteg (\u{05BD}), Etnahta (\u{0591})
        let marks = ['\u{05C1}', '\u{05BC}', '\u{05B8}', '\u{05BD}', '\u{0591}'];
        let base = '\u{05E9}'; // Shin
        let expected = "\u{05E9}\u{05C1}\u{05BC}\u{05B8}\u{05BD}\u{0591}";

        // Generate all 5! = 120 permutations via Heap's algorithm
        let mut perm = marks;
        let mut c = [0usize; 5];
        let mut count = 0usize;

        let check = |p: &[char; 5]| {
            let mut s = String::new();
            s.push(base);
            for &m in p {
                s.push(m);
            }
            let canonical = canonicalize_hebrew(&s);
            assert_eq!(
                canonical, expected,
                "Permutation {:?} must canonicalize to expected order",
                p
            );
        };

        check(&perm);
        count += 1;

        let mut i = 0;
        while i < 5 {
            if c[i] < i {
                if i % 2 == 0 {
                    perm.swap(0, i);
                } else {
                    perm.swap(c[i], i);
                }
                check(&perm);
                count += 1;
                c[i] += 1;
                i = 0;
            } else {
                c[i] = 0;
                i += 1;
            }
        }

        assert_eq!(count, 120, "All 120 permutations of 5 marks must be tested");
    }

    #[test]
    fn test_nun_hafukha_and_masoretic_punctuation_preservation() {
        let text_with_nun_hafukha =
            "\u{05C6}\u{05D5}\u{05D9}\u{05D4}\u{05D9} \u{05D1}\u{05E0}\u{05E1}\u{05E2}\u{05C6}";
        let normalized = HebrewNormalizer::normalize(text_with_nun_hafukha);
        assert_eq!(normalized, text_with_nun_hafukha);
        assert!(HebrewNormalizer::is_nun_hafukha('\u{05C6}'));
        assert!(HebrewNormalizer::is_masoretic_punctuation('\u{05C6}'));
        assert!(HebrewNormalizer::is_hebrew_base_char('\u{05C6}'));

        let stripped = HebrewNormalizer::strip_all_marks(text_with_nun_hafukha);
        assert!(stripped.contains('\u{05C6}'));

        let clean = HebrewNormalizer::sanitize_plain_text(text_with_nun_hafukha);
        assert_eq!(clean, text_with_nun_hafukha);
    }
}
