//! Offline Tanakh Catalog & Local Verse Lookup Engine (נושא 4.2).
//!
//! Provides deterministic offline verification of Tanakh book names/abbreviations,
//! chapter counts across all 24 canonical books (39 divisions), and fast local verse
//! lookup from an embedded core corpus or loaded TSV asset (`book\tchapter\tverse\ttext`).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TanakhDivision {
    Torah,
    Neviim,
    Ketuvim,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TanakhBookMeta {
    pub order: u8,
    pub canonical_name: &'static str,
    pub aliases: &'static [&'static str],
    pub division: TanakhDivision,
    pub chapters: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TanakhLookupResult {
    pub book: String,
    pub chapter: u16,
    pub verse: u16,
    pub text: Option<String>,
    pub valid_chapter: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TanakhLookupError {
    #[error("Unknown Tanakh book: {0}")]
    UnknownBook(String),
    #[error("Chapter {chapter} out of range for {book} (max {max_chapters})")]
    InvalidChapter {
        book: String,
        chapter: u16,
        max_chapters: u16,
    },
    #[error("Verse number must be >= 1")]
    InvalidVerse,
}

pub const TANAKH_CATALOG: &[TanakhBookMeta] = &[
    TanakhBookMeta {
        order: 1,
        canonical_name: "בראשית",
        aliases: &["ברא", "ברא'"],
        division: TanakhDivision::Torah,
        chapters: 50,
    },
    TanakhBookMeta {
        order: 2,
        canonical_name: "שמות",
        aliases: &["שמ", "שמ'"],
        division: TanakhDivision::Torah,
        chapters: 40,
    },
    TanakhBookMeta {
        order: 3,
        canonical_name: "ויקרא",
        aliases: &["וי", "וי'"],
        division: TanakhDivision::Torah,
        chapters: 27,
    },
    TanakhBookMeta {
        order: 4,
        canonical_name: "במדבר",
        aliases: &["במ", "במ'"],
        division: TanakhDivision::Torah,
        chapters: 36,
    },
    TanakhBookMeta {
        order: 5,
        canonical_name: "דברים",
        aliases: &["דב", "דב'"],
        division: TanakhDivision::Torah,
        chapters: 34,
    },
    TanakhBookMeta {
        order: 6,
        canonical_name: "יהושע",
        aliases: &["יהו", "יהו'"],
        division: TanakhDivision::Neviim,
        chapters: 24,
    },
    TanakhBookMeta {
        order: 7,
        canonical_name: "שופטים",
        aliases: &["שופ", "שופ'"],
        division: TanakhDivision::Neviim,
        chapters: 21,
    },
    TanakhBookMeta {
        order: 8,
        canonical_name: "שמואל א",
        aliases: &["ש\"א", "שמואל א'"],
        division: TanakhDivision::Neviim,
        chapters: 31,
    },
    TanakhBookMeta {
        order: 9,
        canonical_name: "שמואל ב",
        aliases: &["ש\"ב", "שמואל ב'"],
        division: TanakhDivision::Neviim,
        chapters: 24,
    },
    TanakhBookMeta {
        order: 10,
        canonical_name: "מלכים א",
        aliases: &["מ\"א", "מלכים א'"],
        division: TanakhDivision::Neviim,
        chapters: 22,
    },
    TanakhBookMeta {
        order: 11,
        canonical_name: "מלכים ב",
        aliases: &["מ\"ב", "מלכים ב'"],
        division: TanakhDivision::Neviim,
        chapters: 25,
    },
    TanakhBookMeta {
        order: 12,
        canonical_name: "ישעיהו",
        aliases: &["ישעיה", "יש'"],
        division: TanakhDivision::Neviim,
        chapters: 66,
    },
    TanakhBookMeta {
        order: 13,
        canonical_name: "ירמיהו",
        aliases: &["ירמיה", "יר'"],
        division: TanakhDivision::Neviim,
        chapters: 52,
    },
    TanakhBookMeta {
        order: 14,
        canonical_name: "יחזקאל",
        aliases: &["יח", "יח'"],
        division: TanakhDivision::Neviim,
        chapters: 48,
    },
    TanakhBookMeta {
        order: 15,
        canonical_name: "הושע",
        aliases: &["הו'"],
        division: TanakhDivision::Neviim,
        chapters: 14,
    },
    TanakhBookMeta {
        order: 16,
        canonical_name: "יואל",
        aliases: &[],
        division: TanakhDivision::Neviim,
        chapters: 4,
    },
    TanakhBookMeta {
        order: 17,
        canonical_name: "עמוס",
        aliases: &[],
        division: TanakhDivision::Neviim,
        chapters: 9,
    },
    TanakhBookMeta {
        order: 18,
        canonical_name: "עובדיה",
        aliases: &[],
        division: TanakhDivision::Neviim,
        chapters: 1,
    },
    TanakhBookMeta {
        order: 19,
        canonical_name: "יונה",
        aliases: &[],
        division: TanakhDivision::Neviim,
        chapters: 4,
    },
    TanakhBookMeta {
        order: 20,
        canonical_name: "מיכה",
        aliases: &[],
        division: TanakhDivision::Neviim,
        chapters: 7,
    },
    TanakhBookMeta {
        order: 21,
        canonical_name: "נחום",
        aliases: &[],
        division: TanakhDivision::Neviim,
        chapters: 3,
    },
    TanakhBookMeta {
        order: 22,
        canonical_name: "חבקוק",
        aliases: &[],
        division: TanakhDivision::Neviim,
        chapters: 3,
    },
    TanakhBookMeta {
        order: 23,
        canonical_name: "צפניה",
        aliases: &[],
        division: TanakhDivision::Neviim,
        chapters: 3,
    },
    TanakhBookMeta {
        order: 24,
        canonical_name: "חגי",
        aliases: &[],
        division: TanakhDivision::Neviim,
        chapters: 2,
    },
    TanakhBookMeta {
        order: 25,
        canonical_name: "זכריה",
        aliases: &[],
        division: TanakhDivision::Neviim,
        chapters: 14,
    },
    TanakhBookMeta {
        order: 26,
        canonical_name: "מלאכי",
        aliases: &[],
        division: TanakhDivision::Neviim,
        chapters: 3,
    },
    TanakhBookMeta {
        order: 27,
        canonical_name: "תהלים",
        aliases: &["תהילים", "תה'"],
        division: TanakhDivision::Ketuvim,
        chapters: 150,
    },
    TanakhBookMeta {
        order: 28,
        canonical_name: "משלי",
        aliases: &["מש'"],
        division: TanakhDivision::Ketuvim,
        chapters: 31,
    },
    TanakhBookMeta {
        order: 29,
        canonical_name: "איוב",
        aliases: &[],
        division: TanakhDivision::Ketuvim,
        chapters: 42,
    },
    TanakhBookMeta {
        order: 30,
        canonical_name: "שיר השירים",
        aliases: &["שה\"ש"],
        division: TanakhDivision::Ketuvim,
        chapters: 8,
    },
    TanakhBookMeta {
        order: 31,
        canonical_name: "רות",
        aliases: &[],
        division: TanakhDivision::Ketuvim,
        chapters: 4,
    },
    TanakhBookMeta {
        order: 32,
        canonical_name: "איכה",
        aliases: &[],
        division: TanakhDivision::Ketuvim,
        chapters: 5,
    },
    TanakhBookMeta {
        order: 33,
        canonical_name: "קהלת",
        aliases: &["קה'"],
        division: TanakhDivision::Ketuvim,
        chapters: 12,
    },
    TanakhBookMeta {
        order: 34,
        canonical_name: "אסתר",
        aliases: &["אס'"],
        division: TanakhDivision::Ketuvim,
        chapters: 10,
    },
    TanakhBookMeta {
        order: 35,
        canonical_name: "דניאל",
        aliases: &["דנ'"],
        division: TanakhDivision::Ketuvim,
        chapters: 12,
    },
    TanakhBookMeta {
        order: 36,
        canonical_name: "עזרא",
        aliases: &["עז'"],
        division: TanakhDivision::Ketuvim,
        chapters: 10,
    },
    TanakhBookMeta {
        order: 37,
        canonical_name: "נחמיה",
        aliases: &["נח'"],
        division: TanakhDivision::Ketuvim,
        chapters: 13,
    },
    TanakhBookMeta {
        order: 38,
        canonical_name: "דברי הימים א",
        aliases: &["דה\"א"],
        division: TanakhDivision::Ketuvim,
        chapters: 29,
    },
    TanakhBookMeta {
        order: 39,
        canonical_name: "דברי הימים ב",
        aliases: &["דה\"ב"],
        division: TanakhDivision::Ketuvim,
        chapters: 36,
    },
];

/// Resolves a Hebrew book name or abbreviation to its canonical [`TanakhBookMeta`].
pub fn resolve_tanakh_book(name: &str) -> Option<&'static TanakhBookMeta> {
    let trimmed = name.trim();
    TANAKH_CATALOG
        .iter()
        .find(|b| b.canonical_name == trimmed || b.aliases.contains(&trimmed))
}

/// Offline Tanakh verse store supporting embedded core verses and TSV bulk loading.
#[derive(Debug, Clone, Default)]
pub struct TanakhLookup {
    verses: HashMap<(String, u16, u16), String>,
}

impl TanakhLookup {
    /// Creates an offline lookup pre-seeded with core foundational verses.
    pub fn with_builtin_core() -> Self {
        let mut lookup = Self::default();
        let builtin = [
            ("בראשית", 1, 1, "בְּרֵאשִׁית בָּרָא אֱלֹהִים אֵת הַשָּׁמַיִם וְאֵת הָאָרֶץ."),
            ("בראשית", 1, 2, "וְהָאָרֶץ הָיְתָה תֹהוּ וָבֹהוּ וְחֹשֶׁךְ עַל פְּנֵי תְהוֹם."),
            (
                "שמות",
                20,
                2,
                "אָנֹכִי ה' אֱלֹהֶיךָ אֲשֶׁר הוֹצֵאתִיךָ מֵאֶרֶץ מִצְרַיִם מִבֵּית עֲבָדִים.",
            ),
            ("דברים", 6, 4, "שְׁמַע יִשְׂרָאֵל ה' אֱלֹהֵינוּ ה' אֶחָד."),
            (
                "יהושע",
                1,
                8,
                "לֹא יָמוּשׁ סֵפֶר הַתּוֹרָה הַזֶּה מִפִּיךָ וְהָגִיתָ בּוֹ יוֹמָם וָלַיְלָה.",
            ),
            ("תהלים", 1, 1, "אַשְׁרֵי הָאִישׁ אֲשֶׁר לֹא הָלַךְ בַּעֲצַת רְשָׁעִים."),
            ("תהלים", 119, 105, "נֵר לְרַגְלִי דְבָרֶךָ וְאוֹר לִנְתִיבָתִי."),
            ("משלי", 1, 7, "יִרְאַת ה' רֵאשִׁית דָּעַת חָכְמָה וּמוּסָר אֱוִילִים בָּזוּ."),
        ];
        for (book, ch, vs, text) in builtin {
            lookup.insert(book, ch, vs, text);
        }
        lookup
    }

    /// Loads verses from a compact TSV string (`book\tchapter\tverse\ttext` per line).
    pub fn from_tsv_str(tsv: &str) -> Self {
        let mut lookup = Self::with_builtin_core();
        for line in tsv.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let mut parts = trimmed.splitn(4, '\t');
            if let (Some(book), Some(ch_s), Some(vs_s), Some(text)) =
                (parts.next(), parts.next(), parts.next(), parts.next())
            {
                if let (Ok(ch), Ok(vs)) = (ch_s.trim().parse::<u16>(), vs_s.trim().parse::<u16>()) {
                    let canon = resolve_tanakh_book(book)
                        .map(|m| m.canonical_name)
                        .unwrap_or_else(|| book.trim());
                    lookup.insert(canon, ch, vs, text.trim());
                }
            }
        }
        lookup
    }

    pub fn insert(&mut self, book: &str, chapter: u16, verse: u16, text: &str) {
        let canon = resolve_tanakh_book(book)
            .map(|m| m.canonical_name.to_string())
            .unwrap_or_else(|| book.trim().to_string());
        self.verses
            .insert((canon, chapter, verse), text.to_string());
    }

    pub fn len(&self) -> usize {
        self.verses.len()
    }

    pub fn is_empty(&self) -> bool {
        self.verses.is_empty()
    }

    /// Verifies book & chapter validity and returns the verse text if present offline.
    pub fn lookup(
        &self,
        book: &str,
        chapter: u16,
        verse: u16,
    ) -> Result<TanakhLookupResult, TanakhLookupError> {
        let meta = resolve_tanakh_book(book)
            .ok_or_else(|| TanakhLookupError::UnknownBook(book.to_string()))?;
        if chapter == 0 || chapter > meta.chapters {
            return Err(TanakhLookupError::InvalidChapter {
                book: meta.canonical_name.to_string(),
                chapter,
                max_chapters: meta.chapters,
            });
        }
        if verse == 0 {
            return Err(TanakhLookupError::InvalidVerse);
        }

        let text = self
            .verses
            .get(&(meta.canonical_name.to_string(), chapter, verse))
            .cloned();

        Ok(TanakhLookupResult {
            book: meta.canonical_name.to_string(),
            chapter,
            verse,
            text,
            valid_chapter: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_offline_tanakh_lookup_and_verification() {
        let db = TanakhLookup::with_builtin_core();
        let res = db.lookup("דב'", 6, 4).unwrap();
        assert_eq!(res.book, "דברים");
        assert!(res.text.unwrap().contains("שְׁמַע יִשְׂרָאֵל"));

        let err = db.lookup("בראשית", 99, 1).unwrap_err();
        assert!(matches!(err, TanakhLookupError::InvalidChapter { .. }));
    }

    #[test]
    fn test_from_tsv_str_loads_custom_verses() {
        let tsv = "שמ'\t1\t1\tוְאֵלֶּה שְׁמוֹת בְּנֵי יִשְׂרָאֵל.\n";
        let db = TanakhLookup::from_tsv_str(tsv);
        let res = db.lookup("שמות", 1, 1).unwrap();
        assert_eq!(res.text.as_deref(), Some("וְאֵלֶּה שְׁמוֹת בְּנֵי יִשְׂרָאֵל."));
    }
}
