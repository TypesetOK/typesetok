//! Canonical Torah Reference Index & Citation Extraction Engine (נושא 4.1).
//!
//! Deterministic regex-based extraction and canonical sorting of Torah citations
//! across Tanakh, Talmud Bavli, Rambam (Mishneh Torah), and Shulchan Aruch.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TorahCorpusKind {
    Tanakh,
    TalmudBavli,
    Rambam,
    ShulchanAruch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TorahCitation {
    pub kind: TorahCorpusKind,
    pub book_or_tractate: String,
    pub chapter_or_daf: u16,
    pub verse_or_amud: Option<String>,
    pub raw_match: String,
    pub canonical_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CitationIndexEntry {
    pub citation: TorahCitation,
    /// `(1-based page number, gematria string)` pairs where this citation appears.
    pub pages: Vec<(usize, String)>,
}

/// Parses a Hebrew gematria or Arabic numeral token (stripping geresh/gershayim).
pub fn parse_hebrew_or_arabic_num(token: &str) -> Option<u16> {
    let clean: String = token
        .chars()
        .filter(|&c| c != '\'' && c != '"' && c != '׳' && c != '״' && !c.is_whitespace())
        .collect();
    if clean.is_empty() {
        return None;
    }
    if let Ok(n) = clean.parse::<u16>() {
        return Some(n);
    }
    let mut sum: u16 = 0;
    for ch in clean.chars() {
        let val = match ch {
            'א' => 1,
            'ב' => 2,
            'ג' => 3,
            'ד' => 4,
            'ה' => 5,
            'ו' => 6,
            'ז' => 7,
            'ח' => 8,
            'ט' => 9,
            'י' => 10,
            'כ' | 'ך' => 20,
            'ל' => 30,
            'מ' | 'ם' => 40,
            'נ' | 'ן' => 50,
            'ס' => 60,
            'ע' => 70,
            'פ' | 'ף' => 80,
            'צ' | 'ץ' => 90,
            'ק' => 100,
            'ר' => 200,
            'ש' => 300,
            'ת' => 400,
            _ => return None,
        };
        sum = sum.saturating_add(val);
    }
    if sum > 0 {
        Some(sum)
    } else {
        None
    }
}

fn bavli_tractates() -> &'static [(&'static str, u16)] {
    &[
        ("ברכות", 1),
        ("שבת", 2),
        ("עירובין", 3),
        ("פסחים", 4),
        ("ראש השנה", 5),
        ("יומא", 6),
        ("סוכה", 7),
        ("ביצה", 8),
        ("תענית", 9),
        ("מגילה", 10),
        ("מועד קטן", 11),
        ("חגיגה", 12),
        ("יבמות", 13),
        ("כתובות", 14),
        ("נדרים", 15),
        ("נזיר", 16),
        ("סוטה", 17),
        ("גיטין", 18),
        ("קידושין", 19),
        ("בבא קמא", 20),
        ("בבא מציעא", 21),
        ("בבא בתרא", 22),
        ("סנהדרין", 23),
        ("מכות", 24),
        ("שבועות", 25),
        ("עבודה זרה", 26),
        ("הוריות", 27),
        ("זבחים", 28),
        ("מנחות", 29),
        ("חולין", 30),
        ("בכורות", 31),
        ("ערכין", 32),
        ("תמורה", 33),
        ("כריתות", 34),
        ("מעילה", 35),
        ("נדה", 36),
    ]
}

fn shulchan_aruch_sections() -> &'static [(&'static str, &'static str, u16)] {
    &[
        ("או\"ח", "אורח חיים", 1),
        ("אורח חיים", "אורח חיים", 1),
        ("יו\"ד", "יורה דעה", 2),
        ("יורה דעה", "יורה דעה", 2),
        ("אה\"ע", "אבן העזר", 3),
        ("אבן העזר", "אבן העזר", 3),
        ("חו\"מ", "חושן משפט", 4),
        ("חושן משפט", "חושן משפט", 4),
    ]
}

fn tanakh_books_prefix() -> &'static [(&'static str, u16)] {
    &[
        ("בראשית", 1),
        ("שמות", 2),
        ("ויקרא", 3),
        ("במדבר", 4),
        ("דברים", 5),
        ("יהושע", 6),
        ("שופטים", 7),
        ("שמואל א", 8),
        ("שמואל ב", 9),
        ("מלכים א", 10),
        ("מלכים ב", 11),
        ("ישעיהו", 12),
        ("ישעיה", 12),
        ("ירמיהו", 13),
        ("ירמיה", 13),
        ("יחזקאל", 14),
        ("תהלים", 15),
        ("תהילים", 15),
        ("משלי", 16),
        ("איוב", 17),
        ("שיר השירים", 18),
        ("רות", 19),
        ("איכה", 20),
        ("קהלת", 21),
        ("אסתר", 22),
        ("דניאל", 23),
        ("עזרא", 24),
        ("נחמיה", 25),
        ("דברי הימים א", 26),
        ("דברי הימים ב", 27),
    ]
}

struct CitationRegexes {
    bavli: Regex,
    shulchan_aruch: Regex,
    tanakh: Regex,
    rambam: Regex,
}

fn citation_regexes() -> &'static CitationRegexes {
    static RE: OnceLock<CitationRegexes> = OnceLock::new();
    RE.get_or_init(|| {
        let tractate_alt = bavli_tractates()
            .iter()
            .map(|(n, _)| regex::escape(n))
            .collect::<Vec<_>>()
            .join("|");
        let bavli = Regex::new(&format!(
            r#"({})\s+(?:דף\s+)?([א-ת0-9'׳"״]{{1,5}})\s*(?:ע[""״']([אב]))?"#,
            tractate_alt
        ))
        .expect("valid bavli regex");

        let sa_alt = shulchan_aruch_sections()
            .iter()
            .map(|(n, _, _)| regex::escape(n))
            .collect::<Vec<_>>()
            .join("|");
        let shulchan_aruch = Regex::new(&format!(
            r#"({})\s+(?:סי['׳]?\s*|סימן\s+)([א-ת0-9'׳"״]{{1,5}})(?:\s+(?:ס['׳]?\s*|סעיף\s+)([א-ת0-9'׳"״]{{1,4}}))?"#,
            sa_alt
        ))
        .expect("valid shulchan aruch regex");

        let tanakh_alt = tanakh_books_prefix()
            .iter()
            .map(|(n, _)| regex::escape(n))
            .collect::<Vec<_>>()
            .join("|");
        let tanakh = Regex::new(&format!(
            r#"({})\s+(?:פרק\s+)?([א-ת0-9'׳"״]{{1,4}})[,:\s]+(?:פסוק\s+)?([א-ת0-9'׳"״]{{1,4}})"#,
            tanakh_alt
        ))
        .expect("valid tanakh regex");

        let rambam = Regex::new(
            r#"רמב["״']ם\s+(?:הל['׳]?\s*|הלכות\s+)([^\s,]+)\s+פ["״']?([א-ת0-9'׳"״]{1,4})\s+ה["״']?([א-ת0-9'׳"״]{1,4})"#,
        )
        .expect("valid rambam regex");

        CitationRegexes {
            bavli,
            shulchan_aruch,
            tanakh,
            rambam,
        }
    })
}

/// Extracts all recognized Torah citations from `text` in deterministic canonical form.
pub fn extract_torah_citations(text: &str) -> Vec<TorahCitation> {
    let re = citation_regexes();
    let mut out = Vec::new();

    for cap in re.bavli.captures_iter(text) {
        let tractate = cap.get(1).map_or("", |m| m.as_str());
        let daf_str = cap.get(2).map_or("", |m| m.as_str());
        let amud = cap.get(3).map(|m| format!("ע\"{}", m.as_str()));
        if let Some(daf) = parse_hebrew_or_arabic_num(daf_str) {
            let order = bavli_tractates()
                .iter()
                .find(|(n, _)| *n == tractate)
                .map_or(999, |(_, o)| *o);
            let amud_key = amud.as_deref().unwrap_or("");
            out.push(TorahCitation {
                kind: TorahCorpusKind::TalmudBavli,
                book_or_tractate: tractate.to_string(),
                chapter_or_daf: daf,
                verse_or_amud: amud.clone(),
                raw_match: cap[0].to_string(),
                canonical_key: format!("2_{:03}_{:04}_{}", order, daf, amud_key),
            });
        }
    }

    for cap in re.shulchan_aruch.captures_iter(text) {
        let sec_raw = cap.get(1).map_or("", |m| m.as_str());
        let siman_str = cap.get(2).map_or("", |m| m.as_str());
        let seif = cap.get(3).map(|m| m.as_str().to_string());
        if let Some(siman) = parse_hebrew_or_arabic_num(siman_str) {
            let (canon_sec, order) = shulchan_aruch_sections()
                .iter()
                .find(|(n, _, _)| *n == sec_raw)
                .map_or((sec_raw, 99), |(_, c, o)| (*c, *o));
            let seif_num = seif
                .as_deref()
                .and_then(parse_hebrew_or_arabic_num)
                .unwrap_or(0);
            out.push(TorahCitation {
                kind: TorahCorpusKind::ShulchanAruch,
                book_or_tractate: canon_sec.to_string(),
                chapter_or_daf: siman,
                verse_or_amud: seif,
                raw_match: cap[0].to_string(),
                canonical_key: format!("4_{:02}_{:04}_{:04}", order, siman, seif_num),
            });
        }
    }

    for cap in re.tanakh.captures_iter(text) {
        let book = cap.get(1).map_or("", |m| m.as_str());
        let ch_str = cap.get(2).map_or("", |m| m.as_str());
        let vs_str = cap.get(3).map_or("", |m| m.as_str());
        if let (Some(ch), Some(vs)) = (
            parse_hebrew_or_arabic_num(ch_str),
            parse_hebrew_or_arabic_num(vs_str),
        ) {
            let order = tanakh_books_prefix()
                .iter()
                .find(|(n, _)| *n == book)
                .map_or(999, |(_, o)| *o);
            out.push(TorahCitation {
                kind: TorahCorpusKind::Tanakh,
                book_or_tractate: book.to_string(),
                chapter_or_daf: ch,
                verse_or_amud: Some(vs_str.to_string()),
                raw_match: cap[0].to_string(),
                canonical_key: format!("1_{:03}_{:04}_{:04}", order, ch, vs),
            });
        }
    }

    for cap in re.rambam.captures_iter(text) {
        let hilchot = cap.get(1).map_or("", |m| m.as_str());
        let ch_str = cap.get(2).map_or("", |m| m.as_str());
        let hal_str = cap.get(3).map_or("", |m| m.as_str());
        if let (Some(ch), Some(hal)) = (
            parse_hebrew_or_arabic_num(ch_str),
            parse_hebrew_or_arabic_num(hal_str),
        ) {
            out.push(TorahCitation {
                kind: TorahCorpusKind::Rambam,
                book_or_tractate: format!("הלכות {}", hilchot),
                chapter_or_daf: ch,
                verse_or_amud: Some(hal_str.to_string()),
                raw_match: cap[0].to_string(),
                canonical_key: format!("3_{}_{:04}_{:04}", hilchot, ch, hal),
            });
        }
    }

    out
}

/// Builds a canonically sorted citation index mapping each unique citation to the pages
/// where it appears.
pub fn build_citation_index<'a>(
    pages: impl Iterator<Item = (usize, &'a str, &'a str)>,
) -> Vec<CitationIndexEntry> {
    let mut map: BTreeMap<String, CitationIndexEntry> = BTreeMap::new();

    for (page_num, gematria, text) in pages {
        for cit in extract_torah_citations(text) {
            let entry =
                map.entry(cit.canonical_key.clone())
                    .or_insert_with(|| CitationIndexEntry {
                        citation: cit,
                        pages: Vec::new(),
                    });
            if !entry.pages.iter().any(|(p, _)| *p == page_num) {
                entry.pages.push((page_num, gematria.to_string()));
            }
        }
    }

    map.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_and_index_torah_citations() {
        let p1 = "כמבואר במסכת ברכות דף ב ע\"א, וכן נפסק בשו\"ע או\"ח סי' רל\"ה סעיף א'.";
        let p2 = "כתיב בראשית א, א וכדברי הרמב\"ם הל' תפילה פ\"א ה\"ב.";
        let index = build_citation_index(vec![(1, "א׳", p1), (2, "ב׳", p2)].into_iter());

        assert_eq!(index.len(), 4);
        // Canonical order: Tanakh (1_) -> Bavli (2_) -> Rambam (3_) -> Shulchan Aruch (4_)
        assert_eq!(index[0].citation.kind, TorahCorpusKind::Tanakh);
        assert_eq!(index[0].citation.book_or_tractate, "בראשית");
        assert_eq!(index[1].citation.kind, TorahCorpusKind::TalmudBavli);
        assert_eq!(index[1].citation.book_or_tractate, "ברכות");
        assert_eq!(index[1].citation.chapter_or_daf, 2);
        assert_eq!(index[2].citation.kind, TorahCorpusKind::Rambam);
        assert_eq!(index[3].citation.kind, TorahCorpusKind::ShulchanAruch);
        assert_eq!(index[3].citation.chapter_or_daf, 235);
    }
}
