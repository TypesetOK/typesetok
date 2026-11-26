pub mod anchor;
pub mod dependency;
pub mod error;
pub mod header;
pub mod hebrew;
pub mod id;
pub mod model;
pub mod multi_flow_anchor;
pub mod normalizer;
pub mod notes;
pub mod numbering;
pub mod outline;
pub mod overrides;
pub mod reference;
pub mod spans;
pub mod styles;
pub mod tanakh;
pub mod torah_index;
pub mod transaction;
pub mod ui_state;

pub use anchor::{AnchorBias, SemanticRange, TextAnchor};
pub use dependency::DocumentDependencyTracker;
pub use error::ModelError;
pub use header::{
    FootnoteApparatusConfig, HeaderExtractionRule, HeaderFolioPosition, HeaderSpreadSide,
    RunningHeaderConfig,
};
pub use id::{FractionalIndex, NodeId, Ulid};
pub use model::{
    DocumentMetadata, DocumentModel, DocumentRoot, Flow, FlowId, FlowType, ImageAttachment,
    OrnamentPlacement, OrnamentRef, ParagraphNode, SectionNode,
};
pub use multi_flow_anchor::{AnchorDefinition, AnchorId, CommentaryLink, MultiFlowAnchorMap};
pub use normalizer::{canonicalize_hebrew, HebrewMarkCategory, HebrewNormalizer};
pub use notes::{
    expand_paragraph_with_inline_notes, InlineNote, Sidenote, SidenoteConfig, SidenoteMarginSide,
};
pub use numbering::{GematriaPunctuation, NumberingFormatter, NumberingScheme, NumberingScope};
pub use outline::{
    DocumentHierarchyNode, DocumentOutline, HierarchyLevel, LevelNumberingRule,
    OutlineNumberingConfig,
};
pub use overrides::{LayoutLockOverrides, PageBreakMode};
pub use reference::{CrossReferenceNode, CrossReferenceRegistry, ReferenceKind, ReferenceTarget};
pub use spans::{
    insert_range, is_canonical, normalize_spans, remove_range, spans_in_range, TextSpan,
};
pub use styles::{
    evaluate_live_grep_spans, AhaltermStretchMode, CharacterFormat, CharacterStyle, Color,
    ColumnDistributionMode, ColumnSpanMode, DropCapConfig, GrepRule, LastLineConstraintConfig,
    OpticalMarginConfig, ParagraphStyle, Progression, StylePatch, TextAlignment,
};
pub use tanakh::{
    resolve_tanakh_book, TanakhBookMeta, TanakhDivision, TanakhLookup, TanakhLookupError,
    TanakhLookupResult, TANAKH_CATALOG,
};
pub use torah_index::{
    build_citation_index, extract_torah_citations, parse_hebrew_or_arabic_num, CitationIndexEntry,
    TorahCitation, TorahCorpusKind,
};
pub use transaction::{AtomicOperation, CompoundTransaction, TransactionStack};
