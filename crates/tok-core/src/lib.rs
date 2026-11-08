pub mod anchor;
pub mod dependency;
pub mod error;
pub mod header;
pub mod hebrew;
pub mod id;
pub mod model;
pub mod multi_flow_anchor;
pub mod normalizer;
pub mod numbering;
pub mod outline;
pub mod overrides;
pub mod reference;
pub mod spans;
pub mod styles;
pub mod transaction;

pub use anchor::{AnchorBias, SemanticRange, TextAnchor};
pub use dependency::DocumentDependencyTracker;
pub use error::ModelError;
pub use header::{
    FootnoteApparatusConfig, HeaderExtractionRule, HeaderFolioPosition, HeaderSpreadSide,
    RunningHeaderConfig,
};
pub use id::{FractionalIndex, NodeId, Ulid};
pub use model::{
    DocumentMetadata, DocumentModel, DocumentRoot, Flow, FlowId, FlowType, ParagraphNode,
    SectionNode,
};
pub use multi_flow_anchor::{AnchorDefinition, AnchorId, CommentaryLink, MultiFlowAnchorMap};
pub use normalizer::{canonicalize_hebrew, HebrewMarkCategory, HebrewNormalizer};
pub use numbering::{GematriaPunctuation, NumberingFormatter, NumberingScheme, NumberingScope};
pub use outline::{
    DocumentHierarchyNode, DocumentOutline, HierarchyLevel, LevelNumberingRule,
    OutlineNumberingConfig,
};
pub use overrides::LayoutLockOverrides;
pub use reference::{CrossReferenceNode, CrossReferenceRegistry, ReferenceKind, ReferenceTarget};
pub use spans::{
    insert_range, is_canonical, normalize_spans, remove_range, spans_in_range, TextSpan,
};
pub use styles::{
    evaluate_live_grep_spans, AhaltermStretchMode, CharacterFormat, CharacterStyle, Color,
    DropCapConfig, GrepRule, LastLineConstraintConfig, OpticalMarginConfig, ParagraphStyle,
    Progression, StylePatch, TextAlignment,
};
pub use transaction::{AtomicOperation, CompoundTransaction, TransactionStack};
