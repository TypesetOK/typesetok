use crate::error::ModelError;
use crate::id::{FractionalIndex, NodeId};
use crate::normalizer::HebrewNormalizer;
use crate::styles::{CharacterStyle, ParagraphStyle, Progression, StylePatch};
use ropey::Rope;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FlowId(pub String);

impl FlowId {
    pub fn main() -> Self {
        Self("main".to_string())
    }

    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl<T: Into<String>> From<T> for FlowId {
    fn from(id: T) -> Self {
        Self(id.into())
    }
}

impl From<&FlowId> for FlowId {
    fn from(id: &FlowId) -> Self {
        id.clone()
    }
}

impl std::fmt::Display for FlowId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FlowType {
    Main,
    CommentA,
    CommentB,
    Footnote,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParagraphNode {
    pub id: NodeId,
    pub index: FractionalIndex,
    pub style_id: String,
    pub text: String,
    #[serde(default)]
    pub style_patches: Vec<StylePatch>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub spans: Vec<crate::spans::TextSpan>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overrides: Option<crate::overrides::LayoutLockOverrides>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heading_level: Option<crate::outline::HierarchyLevel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub numbering_scheme: Option<crate::numbering::NumberingScheme>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number_label: Option<String>,
}

impl ParagraphNode {
    pub fn new(index: FractionalIndex, style_id: impl Into<String>, text: &str) -> Self {
        let normalized = HebrewNormalizer::normalize(text);
        Self {
            id: NodeId::new(),
            index,
            style_id: style_id.into(),
            text: normalized,
            style_patches: Vec::new(),
            spans: Vec::new(),
            overrides: None,
            heading_level: None,
            numbering_scheme: None,
            number_label: None,
        }
    }

    pub fn with_heading_level(mut self, level: crate::outline::HierarchyLevel) -> Self {
        self.heading_level = Some(level);
        self
    }

    pub fn with_numbering_scheme(mut self, scheme: crate::numbering::NumberingScheme) -> Self {
        self.numbering_scheme = Some(scheme);
        self
    }

    pub fn with_number_label(mut self, label: impl Into<String>) -> Self {
        self.number_label = Some(label.into());
        self
    }

    /// Full display title including number label if present (e.g. "סימן א׳: הלכות השכמת הבוקר").
    pub fn full_heading_text(&self) -> String {
        if let Some(lbl) = &self.number_label {
            if self.text.trim().is_empty() {
                lbl.clone()
            } else if self.text.starts_with(lbl) {
                self.text.clone()
            } else {
                format!("{}: {}", lbl, self.text)
            }
        } else {
            self.text.clone()
        }
    }

    pub fn with_spans(mut self, spans: Vec<crate::spans::TextSpan>) -> Self {
        let char_count = self.text.chars().count();
        self.spans = crate::spans::normalize_spans(spans, char_count);
        self
    }

    pub fn with_patch(mut self, patch: StylePatch) -> Self {
        self.style_patches.push(patch);
        self
    }

    pub fn with_drop_cap(self, config: crate::styles::DropCapConfig) -> Self {
        self.with_patch(StylePatch {
            drop_cap: Some(config),
            ..Default::default()
        })
    }

    pub fn with_optical_margin(self, config: crate::styles::OpticalMarginConfig) -> Self {
        self.with_patch(StylePatch {
            optical_margin: Some(config),
            ..Default::default()
        })
    }

    pub fn with_last_line_constraint(
        self,
        config: crate::styles::LastLineConstraintConfig,
    ) -> Self {
        self.with_patch(StylePatch {
            last_line_constraint: Some(config),
            ..Default::default()
        })
    }

    pub fn with_overrides(mut self, overrides: crate::overrides::LayoutLockOverrides) -> Self {
        self.overrides = Some(overrides);
        self
    }

    pub fn effective_style(&self, styles: &[ParagraphStyle]) -> ParagraphStyle {
        let mut base = styles
            .iter()
            .find(|s| s.id == self.style_id)
            .cloned()
            .unwrap_or_default();
        for patch in &self.style_patches {
            base.apply_patch(patch);
        }
        base
    }

    pub fn format_range(
        &mut self,
        start: usize,
        end: usize,
        format: crate::styles::CharacterFormat,
    ) {
        let char_count = self.text.chars().count();
        let s = start.min(char_count);
        let e = end.min(char_count);
        if s < e && !format.is_empty() {
            let mut new_spans = self.spans.clone();
            new_spans.push(crate::spans::TextSpan::new(s, e, format));
            self.spans = crate::spans::normalize_spans(new_spans, char_count);
        }
    }

    pub fn clear_formatting_range(&mut self, start: usize, end: usize) {
        let char_count = self.text.chars().count();
        let s = start.min(char_count);
        let e = end.min(char_count);
        if s < e {
            let mut new_spans = Vec::new();
            for span in &self.spans {
                if span.end <= s || span.start >= e {
                    new_spans.push(span.clone());
                } else {
                    if span.start < s {
                        new_spans.push(crate::spans::TextSpan::new(
                            span.start,
                            s,
                            span.format.clone(),
                        ));
                    }
                    if span.end > e {
                        new_spans.push(crate::spans::TextSpan::new(
                            e,
                            span.end,
                            span.format.clone(),
                        ));
                    }
                }
            }
            self.spans = crate::spans::normalize_spans(new_spans, char_count);
        }
    }

    pub fn is_frozen(&self) -> bool {
        self.overrides.as_ref().is_some_and(|o| o.is_frozen)
    }

    pub fn has_forced_page_break_before(&self) -> bool {
        self.overrides.as_ref().is_some_and(|o| {
            o.force_page_break_before || o.page_break_mode == crate::overrides::PageBreakMode::Page
        })
    }

    pub fn has_forced_odd_page_break_before(&self) -> bool {
        self.overrides.as_ref().is_some_and(|o| {
            o.force_odd_page_break_before
                || o.page_break_mode == crate::overrides::PageBreakMode::OddPage
        })
    }

    pub fn has_forced_page_break_after(&self) -> bool {
        self.overrides
            .as_ref()
            .is_some_and(|o| o.force_page_break_after)
    }

    pub fn rope(&self) -> Rope {
        Rope::from_str(&self.text)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Flow {
    pub id: FlowId,
    pub flow_type: FlowType,
    pub paragraphs: Vec<ParagraphNode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width_ratio: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placement_role: Option<String>,
}

impl Flow {
    pub fn new(id: FlowId, flow_type: FlowType) -> Self {
        Self {
            id,
            flow_type,
            paragraphs: Vec::new(),
            width_ratio: None,
            placement_role: None,
        }
    }

    pub fn with_width_ratio(mut self, ratio: f32) -> Self {
        self.width_ratio = Some(ratio);
        self
    }

    pub fn with_placement_role(mut self, role: impl Into<String>) -> Self {
        self.placement_role = Some(role.into());
        self
    }

    pub fn add_paragraph(&mut self, p: ParagraphNode) {
        if self.paragraphs.is_sorted_by(|a, b| a.index <= b.index) {
            // O(n) insertion after any paragraphs with an equal index
            // (same placement as push + stable sort).
            let pos = self.paragraphs.partition_point(|q| q.index <= p.index);
            self.paragraphs.insert(pos, p);
        } else {
            self.paragraphs.push(p);
            self.paragraphs.sort_by(|a, b| a.index.cmp(&b.index));
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SectionNode {
    pub id: NodeId,
    pub name: String,
    pub page_style: String,
    pub flows: Vec<Flow>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column_proportions: Option<Vec<f32>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expansion_flow_id: Option<FlowId>,
    #[serde(default)]
    pub anchors: crate::multi_flow_anchor::MultiFlowAnchorMap,
    /// Force this section (e.g. Siman or Perek) to start on an odd (recto) page,
    /// automatically injecting a blank verso page if the previous content ends on an odd page.
    #[serde(default)]
    pub start_on_odd_page: bool,
    /// Detailed page break mode for this section.
    #[serde(default)]
    pub page_break_mode: crate::overrides::PageBreakMode,
}

impl SectionNode {
    pub fn new(name: impl Into<String>, page_style: impl Into<String>) -> Self {
        let main_flow = Flow::new(FlowId::main(), FlowType::Main);
        Self {
            id: NodeId::new(),
            name: name.into(),
            page_style: page_style.into(),
            flows: vec![main_flow],
            layout_kind: None,
            column_proportions: None,
            expansion_flow_id: None,
            anchors: crate::multi_flow_anchor::MultiFlowAnchorMap::default(),
            start_on_odd_page: false,
            page_break_mode: crate::overrides::PageBreakMode::Auto,
        }
    }

    /// Sets whether this section must start on an odd (recto) page.
    pub fn with_start_on_odd_page(mut self, start_on_odd: bool) -> Self {
        self.start_on_odd_page = start_on_odd;
        if start_on_odd {
            self.page_break_mode = crate::overrides::PageBreakMode::OddPage;
        }
        self
    }

    /// Sets the page break mode for this section.
    pub fn with_page_break_mode(mut self, mode: crate::overrides::PageBreakMode) -> Self {
        self.page_break_mode = mode;
        if mode == crate::overrides::PageBreakMode::OddPage {
            self.start_on_odd_page = true;
        }
        self
    }

    pub fn with_anchors(mut self, anchors: crate::multi_flow_anchor::MultiFlowAnchorMap) -> Self {
        self.anchors = anchors;
        self
    }

    pub fn with_layout_kind(mut self, kind: impl Into<String>) -> Self {
        self.layout_kind = Some(kind.into());
        self
    }

    pub fn with_column_proportions(mut self, proportions: Vec<f32>) -> Self {
        self.column_proportions = Some(proportions);
        self
    }

    pub fn with_expansion_flow_id(mut self, id: FlowId) -> Self {
        self.expansion_flow_id = Some(id);
        self
    }

    pub fn main_flow_mut(&mut self) -> Option<&mut Flow> {
        self.flows
            .iter_mut()
            .find(|f| f.id == FlowId::main() || f.id.0 == "gemara" || f.flow_type == FlowType::Main)
    }

    pub fn main_flow(&self) -> Option<&Flow> {
        self.flows
            .iter()
            .find(|f| f.id == FlowId::main() || f.id.0 == "gemara" || f.flow_type == FlowType::Main)
    }

    pub fn with_flows(mut self, flows: Vec<Flow>) -> Self {
        self.flows = flows;
        self
    }

    pub fn with_flow(mut self, flow: Flow) -> Self {
        self.flows.push(flow);
        self
    }

    pub fn find_flow(&self, id: &str) -> Option<&Flow> {
        self.flows.iter().find(|f| f.id.0 == id)
    }

    pub fn find_flow_mut(&mut self, id: &str) -> Option<&mut Flow> {
        self.flows.iter_mut().find(|f| f.id.0 == id)
    }

    pub fn has_flow(&self, id: &str) -> bool {
        self.flows.iter().any(|f| f.id.0 == id)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub title: String,
    pub author: String,
    pub progression: Progression,
    pub primary_language: String,
    pub schema_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document_settings: Option<serde_json::Value>,
}

impl Default for DocumentMetadata {
    fn default() -> Self {
        Self {
            title: "מסמך חדש".to_string(),
            author: "מחבר".to_string(),
            progression: Progression::Rtl,
            primary_language: "he".to_string(),
            schema_version: "1.0".to_string(),
            document_settings: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentRoot {
    pub id: NodeId,
    pub metadata: DocumentMetadata,
    pub paragraph_styles: Vec<ParagraphStyle>,
    pub character_styles: Vec<CharacterStyle>,
    pub sections: Vec<SectionNode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outline: Option<crate::outline::DocumentOutline>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cross_references: Option<crate::reference::CrossReferenceRegistry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub running_header: Option<crate::header::RunningHeaderConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub footnote_apparatus: Option<crate::header::FootnoteApparatusConfig>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub footnote_apparatuses: Vec<crate::header::FootnoteApparatusConfig>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub grep_rules: Vec<crate::styles::GrepRule>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dependency_tracker: Option<crate::dependency::DocumentDependencyTracker>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub multi_flow_anchors: Option<crate::multi_flow_anchor::MultiFlowAnchorMap>,
}

impl DocumentRoot {
    pub fn new(title: impl Into<String>) -> Self {
        let mut root = Self {
            id: NodeId::new(),
            metadata: DocumentMetadata {
                title: title.into(),
                ..Default::default()
            },
            paragraph_styles: vec![ParagraphStyle::default()],
            character_styles: Vec::new(),
            sections: Vec::new(),
            outline: None,
            cross_references: None,
            running_header: None,
            footnote_apparatus: None,
            footnote_apparatuses: Vec::new(),
            grep_rules: Vec::new(),
            dependency_tracker: None,
            multi_flow_anchors: None,
        };

        // Add a default first section
        let default_section = SectionNode::new("שער ראשון", "chapter-first");
        root.sections.push(default_section);
        root
    }

    pub fn with_outline(mut self, outline: crate::outline::DocumentOutline) -> Self {
        self.outline = Some(outline);
        self
    }

    pub fn with_cross_references(mut self, refs: crate::reference::CrossReferenceRegistry) -> Self {
        self.cross_references = Some(refs);
        self
    }

    pub fn with_running_header(mut self, header: crate::header::RunningHeaderConfig) -> Self {
        self.running_header = Some(header);
        self
    }

    pub fn with_footnote_apparatus(
        mut self,
        apparatus: crate::header::FootnoteApparatusConfig,
    ) -> Self {
        self.footnote_apparatus = Some(apparatus);
        self
    }

    pub fn with_footnote_apparatuses(
        mut self,
        apparatuses: Vec<crate::header::FootnoteApparatusConfig>,
    ) -> Self {
        self.footnote_apparatuses = apparatuses;
        self
    }

    /// Synchronizes outline numbering across all nodes and all linked or matching ParagraphNodes.
    /// Returns a list of (NodeId, String) representing (paragraph_id, new_number_label).
    pub fn sync_outline_numbering(
        &mut self,
        config: &crate::outline::OutlineNumberingConfig,
    ) -> Vec<(NodeId, String)> {
        let mut affected = Vec::new();
        if let Some(outline) = &mut self.outline {
            outline.recalculate_numbers(config);

            let mut label_map: std::collections::HashMap<
                NodeId,
                (
                    crate::outline::HierarchyLevel,
                    Option<crate::numbering::NumberingScheme>,
                    Option<String>,
                ),
            > = std::collections::HashMap::new();

            for (node, _) in outline.flatten_in_order() {
                let target_p_id = node
                    .target_paragraph_id
                    .or_else(|| node.start_anchor.as_ref().map(|a| a.node_id));
                if let Some(pid) = target_p_id {
                    let scheme = node
                        .numbering_scheme
                        .clone()
                        .or_else(|| config.scheme_for_level(&node.level));
                    label_map.insert(pid, (node.level.clone(), scheme, node.number_label.clone()));
                }
            }

            for sec in &mut self.sections {
                for flow in &mut sec.flows {
                    for p in &mut flow.paragraphs {
                        if let Some((lvl, scheme, label)) = label_map.get(&p.id) {
                            p.heading_level = Some(lvl.clone());
                            p.numbering_scheme = scheme.clone();
                            p.number_label = label.clone();
                            if let Some(lbl) = label {
                                affected.push((p.id, lbl.clone()));
                            }
                        }
                    }
                }
            }
        }
        affected
    }

    pub fn add_footnote_apparatus(&mut self, apparatus: crate::header::FootnoteApparatusConfig) {
        if let Some(pos) = self
            .footnote_apparatuses
            .iter()
            .position(|a| a.id == apparatus.id)
        {
            self.footnote_apparatuses[pos] = apparatus;
        } else {
            self.footnote_apparatuses.push(apparatus);
        }
    }

    pub fn apparatuses(&self) -> Vec<crate::header::FootnoteApparatusConfig> {
        if !self.footnote_apparatuses.is_empty() {
            self.footnote_apparatuses.clone()
        } else if let Some(app) = &self.footnote_apparatus {
            vec![app.clone()]
        } else {
            Vec::new()
        }
    }

    pub fn find_apparatus(&self, id: &str) -> Option<&crate::header::FootnoteApparatusConfig> {
        if let Some(app) = self
            .footnote_apparatuses
            .iter()
            .find(|a| a.id == id || a.flow_id.as_deref() == Some(id))
        {
            Some(app)
        } else if let Some(app) = &self.footnote_apparatus {
            if app.id == id
                || app.flow_id.as_deref() == Some(id)
                || id == "footnote"
                || id == "notes"
            {
                Some(app)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn format_footnote_marker(&self, apparatus_id: &str, index: usize) -> String {
        if let Some(app) = self.find_apparatus(apparatus_id) {
            app.format_marker(index)
        } else {
            index.to_string()
        }
    }

    pub fn with_grep_rules(mut self, rules: Vec<crate::styles::GrepRule>) -> Self {
        self.grep_rules = rules;
        self
    }

    pub fn add_grep_rule(&mut self, rule: crate::styles::GrepRule) {
        if let Some(pos) = self.grep_rules.iter().position(|r| r.id == rule.id) {
            self.grep_rules[pos] = rule;
        } else {
            self.grep_rules.push(rule);
        }
    }

    /// Adds or updates a `CharacterStyle` in the document.
    pub fn add_character_style(&mut self, style: crate::styles::CharacterStyle) {
        if let Some(pos) = self.character_styles.iter().position(|s| s.id == style.id) {
            self.character_styles[pos] = style;
        } else {
            self.character_styles.push(style);
        }
    }

    /// Adds or updates a `ParagraphStyle` in the document.
    pub fn add_paragraph_style(&mut self, style: crate::styles::ParagraphStyle) {
        if let Some(pos) = self.paragraph_styles.iter().position(|s| s.id == style.id) {
            self.paragraph_styles[pos] = style;
        } else {
            self.paragraph_styles.push(style);
        }
    }

    /// Evaluates active non-destructive Live GREP rules against `paragraph` and merges
    /// them with explicit `paragraph.spans`.
    pub fn effective_spans_for_paragraph(
        &self,
        paragraph: &ParagraphNode,
        flow_id: Option<&str>,
    ) -> Vec<crate::spans::TextSpan> {
        crate::styles::evaluate_live_grep_spans(
            &paragraph.text,
            &paragraph.spans,
            &self.grep_rules,
            &self.character_styles,
            flow_id,
        )
    }

    pub fn with_dependency_tracker(
        mut self,
        tracker: crate::dependency::DocumentDependencyTracker,
    ) -> Self {
        self.dependency_tracker = Some(tracker);
        self
    }

    pub fn find_paragraph(&self, id: NodeId) -> Option<&ParagraphNode> {
        for sec in &self.sections {
            for flow in &sec.flows {
                for p in &flow.paragraphs {
                    if p.id == id {
                        return Some(p);
                    }
                }
            }
        }
        None
    }

    pub fn find_paragraph_mut(&mut self, id: NodeId) -> Option<&mut ParagraphNode> {
        for sec in &mut self.sections {
            for flow in &mut sec.flows {
                for p in &mut flow.paragraphs {
                    if p.id == id {
                        return Some(p);
                    }
                }
            }
        }
        None
    }

    pub fn to_json(&self) -> Result<String, ModelError> {
        serde_json::to_string_pretty(self)
            .map_err(|e| ModelError::SerializationError(e.to_string()))
    }

    pub fn from_json(json_str: &str) -> Result<Self, ModelError> {
        serde_json::from_str(json_str).map_err(|e| ModelError::SerializationError(e.to_string()))
    }
}

/// The document model wrapper that handles snapshots and concurrency.
#[derive(Debug, Clone)]
pub struct DocumentModel {
    root: Arc<DocumentRoot>,
}

impl DocumentModel {
    pub fn new(root: DocumentRoot) -> Self {
        Self {
            root: Arc::new(root),
        }
    }

    pub fn snapshot(&self) -> Arc<DocumentRoot> {
        Arc::clone(&self.root)
    }

    pub fn update_root(&mut self, new_root: DocumentRoot) {
        self.root = Arc::new(new_root);
    }

    pub fn root(&self) -> &DocumentRoot {
        &self.root
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::FractionalIndex;

    #[test]
    fn test_add_paragraph_maintains_order() {
        let mut flow = Flow::new(FlowId::main(), FlowType::Main);
        let p3 = ParagraphNode::new(FractionalIndex::new("p"), "normal", "third");
        let p1 = ParagraphNode::new(FractionalIndex::new("b"), "normal", "first");
        let p2 = ParagraphNode::new(FractionalIndex::new("m"), "normal", "second");

        // Add in shuffled order
        flow.add_paragraph(p3);
        flow.add_paragraph(p1);
        flow.add_paragraph(p2);

        // They should be sorted by index
        assert_eq!(flow.paragraphs[0].text, "first");
        assert_eq!(flow.paragraphs[1].text, "second");
        assert_eq!(flow.paragraphs[2].text, "third");
    }

    #[test]
    fn test_document_to_json_and_back() {
        let mut root = DocumentRoot::new("Test Doc");
        let sec = &mut root.sections[0];
        let flow = sec.main_flow_mut().unwrap();
        flow.add_paragraph(ParagraphNode::new(
            FractionalIndex::initial(),
            "normal",
            "Hello World",
        ));

        let json = root.to_json().expect("Serialization must succeed");
        let loaded = DocumentRoot::from_json(&json).expect("Deserialization must succeed");
        assert_eq!(loaded.metadata.title, "Test Doc");
        assert_eq!(loaded.sections[0].main_flow().unwrap().paragraphs.len(), 1);
    }

    #[test]
    fn test_multi_flow_document_serialization_roundtrip() {
        let mut root = DocumentRoot::new("דף גמרא - ברכות");
        root.metadata.progression = Progression::Rtl;
        root.metadata.primary_language = "he".to_string();

        let sec = &mut root.sections[0];
        sec.flows.clear();

        // Main Gemara flow
        let mut gemara_flow = Flow::new(FlowId::new("gemara"), FlowType::Main);
        gemara_flow.add_paragraph(ParagraphNode::new(
            FractionalIndex::new("a0"),
            "style-gemara-main",
            "מֵאֵימָתַי קוֹרִין אֶת שְׁמַע בְּעַרְבִית?",
        ));
        let gemara_expected_text = gemara_flow.paragraphs[0].text.clone();
        sec.flows.push(gemara_flow);

        // Rashi commentary flow
        let mut rashi_flow = Flow::new(FlowId::new("rashi"), FlowType::CommentA);
        rashi_flow.add_paragraph(ParagraphNode::new(
            FractionalIndex::new("a0"),
            "style-rashi-body",
            "תַּנָּא אַקְּרָא קָאֵי דִּכְתִיב בְּשָׁכְבְּךָ וּבְקוּמֶךָ",
        ));
        let rashi_expected_text = rashi_flow.paragraphs[0].text.clone();
        sec.flows.push(rashi_flow);

        // Tosafot commentary flow
        let mut tosafot_flow = Flow::new(FlowId::new("tosafot"), FlowType::CommentB);
        tosafot_flow.add_paragraph(ParagraphNode::new(
            FractionalIndex::new("a0"),
            "style-tosafot-body",
            "פֵּרֵשׁ רַשִׁ\"י דְּתַנָּא אַקְּרָא קָאֵי",
        ));
        sec.flows.push(tosafot_flow);

        // Notes flow
        let mut notes_flow = Flow::new(FlowId::new("notes"), FlowType::Footnote);
        notes_flow.add_paragraph(ParagraphNode::new(
            FractionalIndex::new("a0"),
            "style-footnotes",
            "תורה אור: דברים ו, ז.",
        ));
        sec.flows.push(notes_flow);

        let json = root
            .to_json()
            .expect("Multi-flow serialization must succeed");
        let restored = DocumentRoot::from_json(&json).expect("Deserialization must succeed");

        assert_eq!(restored.metadata.title, "דף גמרא - ברכות");
        assert_eq!(restored.metadata.progression, Progression::Rtl);
        assert_eq!(restored.sections.len(), 1);
        assert_eq!(restored.sections[0].flows.len(), 4);

        assert_eq!(restored.sections[0].flows[0].id.0, "gemara");
        assert_eq!(
            restored.sections[0].flows[0].paragraphs[0].text,
            gemara_expected_text
        );

        assert_eq!(restored.sections[0].flows[1].id.0, "rashi");
        assert_eq!(
            restored.sections[0].flows[1].paragraphs[0].text,
            rashi_expected_text
        );
        assert_eq!(restored.sections[0].flows[2].id.0, "tosafot");
        assert_eq!(restored.sections[0].flows[3].id.0, "notes");

        // Verify main_flow() finds the gemara flow
        assert!(restored.sections[0].main_flow().is_some());
        assert_eq!(restored.sections[0].main_flow().unwrap().id.0, "gemara");
    }

    #[test]
    fn test_hebrew_niqqud_preservation_in_document() {
        let text_with_niqqud = "שָׁלוֹם עֲלֵיכֶם מַלְאֲכֵי הַשָּׁרֵת מַלְאֲכֵי עֶלְיוֹן";
        let p = ParagraphNode::new(FractionalIndex::initial(), "default-body", text_with_niqqud);
        assert_eq!(p.text, HebrewNormalizer::normalize(text_with_niqqud));

        let mut root = DocumentRoot::new("מנוקד");
        root.sections[0]
            .main_flow_mut()
            .unwrap()
            .add_paragraph(p.clone());

        let json = root.to_json().unwrap();
        let loaded = DocumentRoot::from_json(&json).unwrap();
        let loaded_text = &loaded.sections[0].main_flow().unwrap().paragraphs[0].text;
        assert_eq!(loaded_text, &p.text);
    }

    #[test]
    fn test_empty_document_root() {
        let root = DocumentRoot::new("");
        let json = root.to_json().unwrap();
        let loaded = DocumentRoot::from_json(&json).unwrap();
        assert_eq!(loaded.metadata.title, "");
        assert_eq!(loaded.sections.len(), 1);
        assert_eq!(loaded.sections[0].flows[0].paragraphs.len(), 0);
    }

    #[test]
    fn test_flow_layout_constraints_serialization_roundtrip() {
        let mut root = DocumentRoot::new("תבנית רב-זרימתית");
        let sec = &mut root.sections[0];
        *sec = SectionNode::new("פרק ראשון", "talmud-page")
            .with_layout_kind("dynamic_talmud")
            .with_column_proportions(vec![0.40, 0.28, 0.32])
            .with_expansion_flow_id(FlowId::new("rashi"));

        let flow1 = Flow::new(FlowId::new("main"), FlowType::Main)
            .with_width_ratio(0.40)
            .with_placement_role("primary");
        let flow2 = Flow::new(FlowId::new("rashi"), FlowType::CommentA)
            .with_width_ratio(0.28)
            .with_placement_role("inner_spine");
        let flow3 = Flow::new(FlowId::new("tosafot"), FlowType::CommentB)
            .with_width_ratio(0.32)
            .with_placement_role("outer_margin");

        sec.flows = vec![flow1, flow2, flow3];

        let json = root.to_json().expect("Serialization must succeed");
        let loaded = DocumentRoot::from_json(&json).expect("Deserialization must succeed");

        let loaded_sec = &loaded.sections[0];
        assert_eq!(loaded_sec.layout_kind.as_deref(), Some("dynamic_talmud"));
        assert_eq!(loaded_sec.column_proportions, Some(vec![0.40, 0.28, 0.32]));
        assert_eq!(loaded_sec.expansion_flow_id, Some(FlowId::new("rashi")));

        assert_eq!(loaded_sec.flows[0].width_ratio, Some(0.40));
        assert_eq!(
            loaded_sec.flows[0].placement_role.as_deref(),
            Some("primary")
        );
        assert_eq!(loaded_sec.flows[1].width_ratio, Some(0.28));
        assert_eq!(
            loaded_sec.flows[1].placement_role.as_deref(),
            Some("inner_spine")
        );
        assert_eq!(loaded_sec.flows[2].width_ratio, Some(0.32));
        assert_eq!(
            loaded_sec.flows[2].placement_role.as_deref(),
            Some("outer_margin")
        );
    }

    #[test]
    fn test_flow_layout_backwards_compatibility() {
        let legacy_json = r#"{
            "id": "01ARZ3NDEKTSV4RRFFQ69G5FAV",
            "metadata": {
                "title": "ישן",
                "author": "מחבר",
                "progression": "Rtl",
                "primary_language": "he",
                "schema_version": "1.0"
            },
            "paragraph_styles": [],
            "character_styles": [],
            "sections": [
                {
                    "id": "01ARZ3NDEKTSV4RRFFQ69G5FAV",
                    "name": "שער",
                    "page_style": "default",
                    "flows": [
                        {
                            "id": "main",
                            "flow_type": "Main",
                            "paragraphs": []
                        }
                    ]
                }
            ]
        }"#;

        let loaded = DocumentRoot::from_json(legacy_json).expect("Legacy JSON must parse");
        let sec = &loaded.sections[0];
        assert!(sec.layout_kind.is_none());
        assert!(sec.column_proportions.is_none());
        assert!(sec.expansion_flow_id.is_none());
        assert!(sec.flows[0].width_ratio.is_none());
        assert!(sec.flows[0].placement_role.is_none());
    }

    #[test]
    fn test_section_node_builder_and_query_methods() {
        let sec = SectionNode::new("פרק ראשון", "default")
            .with_layout_kind("parallel_columns")
            .with_column_proportions(vec![0.6, 0.4])
            .with_expansion_flow_id(FlowId::new("commentary"))
            .with_flows(vec![
                Flow::new(FlowId::new("stream_a"), FlowType::Main),
                Flow::new(FlowId::new("stream_b"), FlowType::CommentA),
            ])
            .with_flow(Flow::new(FlowId::new("stream_c"), FlowType::Footnote));

        assert_eq!(sec.flows.len(), 3);
        assert!(sec.has_flow("stream_a"));
        assert!(sec.has_flow("stream_b"));
        assert!(sec.has_flow("stream_c"));
        assert!(!sec.has_flow("stream_d"));
        assert_eq!(
            sec.find_flow("stream_b").unwrap().flow_type,
            FlowType::CommentA
        );
        assert_eq!(sec.layout_kind.as_deref(), Some("parallel_columns"));
        assert_eq!(sec.column_proportions, Some(vec![0.6, 0.4]));
    }

    #[test]
    fn test_paragraph_spans_serialization_and_range_formatting() {
        use crate::spans::TextSpan;
        use crate::styles::CharacterFormat;

        let mut p = ParagraphNode::new(FractionalIndex::initial(), "body", "שלום עולם יפה");
        assert_eq!(p.spans.len(), 0);

        // Format "עולם" (chars 5..9) as bold
        p.format_range(
            5,
            9,
            CharacterFormat {
                font_weight: Some(700),
                ..Default::default()
            },
        );
        assert_eq!(
            p.spans,
            vec![TextSpan::new(
                5,
                9,
                CharacterFormat {
                    font_weight: Some(700),
                    ..Default::default()
                }
            )]
        );

        // Format "יפה" (chars 10..13) as italic
        p.format_range(
            10,
            13,
            CharacterFormat {
                italic: Some(true),
                ..Default::default()
            },
        );
        assert_eq!(p.spans.len(), 2);

        // Serialize and deserialize roundtrip
        let json = serde_json::to_string(&p).expect("serialization must succeed");
        assert!(json.contains(r#""spans""#));
        let deserialized: ParagraphNode =
            serde_json::from_str(&json).expect("deserialization must succeed");
        assert_eq!(deserialized.spans, p.spans);

        // Clear formatting partially across "עולם"
        p.clear_formatting_range(6, 9);
        assert_eq!(
            p.spans[0],
            TextSpan::new(
                5,
                6,
                CharacterFormat {
                    font_weight: Some(700),
                    ..Default::default()
                }
            )
        );

        // Backwards compatibility: paragraph without spans field deserializes with empty spans
        let legacy_json = r#"{"id":"01ARZ3NDEKTSV4RRFFQ69G5FAV","index":"a00000","style_id":"body","text":"טקסט ישן"}"#;
        let legacy_node: ParagraphNode =
            serde_json::from_str(legacy_json).expect("legacy json must parse");
        assert_eq!(legacy_node.spans, Vec::<TextSpan>::new());
    }

    #[test]
    fn test_paragraph_effective_style_with_patches() {
        use crate::styles::{ParagraphStyle, StylePatch, TextAlignment};

        let base_style = ParagraphStyle {
            id: "heading".to_string(),
            font_size_pt: 16.0,
            alignment: TextAlignment::Right,
            ..Default::default()
        };
        let styles = vec![base_style];

        let mut p = ParagraphNode::new(FractionalIndex::initial(), "heading", "כותרת");
        assert_eq!(p.effective_style(&styles).font_size_pt, 16.0);
        assert_eq!(p.effective_style(&styles).alignment, TextAlignment::Right);

        // Add a patch changing alignment to Center and font size to 20
        p = p.with_patch(StylePatch {
            font_size_pt: Some(20.0),
            alignment: Some(TextAlignment::Center),
            ..Default::default()
        });

        let effective = p.effective_style(&styles);
        assert_eq!(effective.font_size_pt, 20.0);
        assert_eq!(effective.alignment, TextAlignment::Center);
    }

    #[test]
    fn test_document_root_multiple_footnote_apparatuses() {
        use crate::header::FootnoteApparatusConfig;
        use crate::numbering::{GematriaPunctuation, NumberingScheme};

        let mut doc = DocumentRoot::new("ספר שו\"ת");
        let sources = FootnoteApparatusConfig::new("sources")
            .with_title("מקורות")
            .with_columns(2, 10.0)
            .with_numbering(NumberingScheme::HebrewGematria(
                GematriaPunctuation::PlainLetters,
            ))
            .with_marker_delimiters("[", "]");

        let biurim = FootnoteApparatusConfig::new("biurim")
            .with_title("ביאורים")
            .with_columns(1, 12.0)
            .with_numbering(NumberingScheme::Arabic)
            .with_marker_delimiters("(", ")");

        doc.add_footnote_apparatus(sources);
        doc.add_footnote_apparatus(biurim);

        assert_eq!(doc.apparatuses().len(), 2);
        assert_eq!(doc.format_footnote_marker("sources", 1), "[א]");
        assert_eq!(doc.format_footnote_marker("biurim", 1), "(1)");
        assert_eq!(doc.find_apparatus("sources").unwrap().columns, 2);
        assert_eq!(doc.find_apparatus("biurim").unwrap().columns, 1);
    }

    #[test]
    fn test_sync_outline_numbering_to_paragraphs() {
        use crate::numbering::GematriaPunctuation;
        use crate::outline::{
            DocumentHierarchyNode, DocumentOutline, HierarchyLevel, OutlineNumberingConfig,
        };

        let mut doc = DocumentRoot::new("שו\"ת מהר\"ם");
        let flow = doc.sections[0].main_flow_mut().unwrap();

        let p1 = ParagraphNode::new(FractionalIndex::initial(), "heading-siman", "הלכות שבת");
        let p1_id = p1.id;
        let p2 = ParagraphNode::new(FractionalIndex::initial(), "heading-seif", "דיני הבדלה");
        let p2_id = p2.id;
        let p3 = ParagraphNode::new(FractionalIndex::initial(), "heading-seif", "כוס של ברכה");
        let p3_id = p3.id;

        flow.paragraphs.push(p1);
        flow.paragraphs.push(p2);
        flow.paragraphs.push(p3);

        let mut outline = DocumentOutline::new();
        let mut siman_node = DocumentHierarchyNode::new(HierarchyLevel::Chapter, "הלכות שבת")
            .with_target_paragraph(p1_id);
        let seif1_node = DocumentHierarchyNode::new(HierarchyLevel::Section, "דיני הבדלה")
            .with_target_paragraph(p2_id);
        let seif2_node = DocumentHierarchyNode::new(HierarchyLevel::Section, "כוס של ברכה")
            .with_target_paragraph(p3_id);

        siman_node.add_child(seif1_node);
        siman_node.add_child(seif2_node);
        outline.add_root_node(siman_node);

        doc.outline = Some(outline);

        let config =
            OutlineNumberingConfig::torah_default(GematriaPunctuation::StandardPunctuation);
        let affected = doc.sync_outline_numbering(&config);

        assert_eq!(affected.len(), 3);

        let main_flow = doc.sections[0].main_flow().unwrap();
        assert_eq!(
            main_flow.paragraphs[0].number_label.as_deref(),
            Some("סימן א׳")
        );
        assert_eq!(
            main_flow.paragraphs[0].full_heading_text(),
            "סימן א׳: הלכות שבת"
        );

        assert_eq!(
            main_flow.paragraphs[1].number_label.as_deref(),
            Some("סעיף א׳")
        );
        assert_eq!(
            main_flow.paragraphs[1].full_heading_text(),
            "סעיף א׳: דיני הבדלה"
        );

        assert_eq!(
            main_flow.paragraphs[2].number_label.as_deref(),
            Some("סעיף ב׳")
        );
        assert_eq!(
            main_flow.paragraphs[2].full_heading_text(),
            "סעיף ב׳: כוס של ברכה"
        );
    }
}
