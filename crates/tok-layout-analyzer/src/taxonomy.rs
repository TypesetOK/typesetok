//! Page layout taxonomy and mapping to TypesetOK core types.
//!
//! Defines the six canonical [`PageForm`] classes identified in the research
//! architecture and maps them cleanly to `tok-ui` `TemplateType` identifiers
//! and `tok_typeset::LayoutFamily`.

use serde::{Deserialize, Serialize};

/// Canonical page-form classification returned by the layout analyzer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PageForm {
    /// 1. Standard prose / continuous running text (`'prose'`, `SingleFlow`).
    StandardProse,
    /// 2. Study/Halacha book with bottom footnote band (`'notes'`, `FootnotesBand`).
    ProseWithFootnotes,
    /// 3. Talmud / Gemara — Tzurat HaDaf with L-shaped commentary wrapping (`'gemara'`, `TzuratHaDaf`).
    TzuratHaDaf,
    /// 4. Mikraot Gedolot / Shulchan Aruch multi-commentary grid (`'mikraot'`, `ParallelColumns >= 4`).
    MultiCommentaryGrid,
    /// 5. Academic / bulletin / modern multi-column layout (`'bulletin'`, `ParallelColumns 2..=6`).
    MultiColumnModern,
    /// 6. Title page, chapter opening, or sparse outlier page (does not override document template).
    TitleOrOutlierPage,
}

impl PageForm {
    /// Returns all six taxonomy variants in canonical order.
    pub const fn all() -> [Self; 6] {
        [
            Self::StandardProse,
            Self::ProseWithFootnotes,
            Self::TzuratHaDaf,
            Self::MultiCommentaryGrid,
            Self::MultiColumnModern,
            Self::TitleOrOutlierPage,
        ]
    }

    /// Maps to the `tok-ui` `TemplateType` string (`'prose' | 'notes' | 'gemara' | 'mikraot' | 'bulletin'`).
    /// Returns `None` for [`PageForm::TitleOrOutlierPage`] since outliers do not select a document template.
    pub const fn to_ui_template_type(self) -> Option<&'static str> {
        match self {
            Self::StandardProse => Some("prose"),
            Self::ProseWithFootnotes => Some("notes"),
            Self::TzuratHaDaf => Some("gemara"),
            Self::MultiCommentaryGrid => Some("mikraot"),
            Self::MultiColumnModern => Some("bulletin"),
            Self::TitleOrOutlierPage => None,
        }
    }

    /// Canonical ASCII identifier for serialization and logging.
    pub const fn id_name(self) -> &'static str {
        match self {
            Self::StandardProse => "standard_prose",
            Self::ProseWithFootnotes => "prose_with_footnotes",
            Self::TzuratHaDaf => "tzurat_hadaf",
            Self::MultiCommentaryGrid => "multi_commentary_grid",
            Self::MultiColumnModern => "multi_column_modern",
            Self::TitleOrOutlierPage => "title_or_outlier",
        }
    }

    /// Human-readable Hebrew display name for UI badges and reports.
    pub const fn display_name_he(self) -> &'static str {
        match self {
            Self::StandardProse => "ספר פרוזה / טקסט רציף",
            Self::ProseWithFootnotes => "ספר עם הערות שוליים",
            Self::TzuratHaDaf => "צורת הדף (גמרא ומפרשים)",
            Self::MultiCommentaryGrid => "מקראות גדולות / רב-נושאי כלים",
            Self::MultiColumnModern => "רב-טורי / עלון / כתב עת",
            Self::TitleOrOutlierPage => "עמוד שער / פתיחת פרק",
        }
    }

    /// Whether this form represents an outlier page that should be excluded from body voting.
    pub const fn is_outlier(self) -> bool {
        matches!(self, Self::TitleOrOutlierPage)
    }
}
