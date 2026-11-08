use crate::anchor::TextAnchor;
use crate::id::NodeId;
use crate::numbering::{GematriaPunctuation, NumberingFormatter, NumberingScheme, NumberingScope};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Structural level of a division in a hierarchical document or multi-volume book.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HierarchyLevel {
    Volume,
    Part,
    Chapter,
    Section,
    Subsection,
    Clause,
    Custom(String),
}

impl HierarchyLevel {
    pub fn default_depth(&self) -> usize {
        match self {
            HierarchyLevel::Volume => 0,
            HierarchyLevel::Part => 1,
            HierarchyLevel::Chapter => 2,
            HierarchyLevel::Section => 3,
            HierarchyLevel::Subsection => 4,
            HierarchyLevel::Clause => 5,
            HierarchyLevel::Custom(_) => 3,
        }
    }
}

/// A node within the hierarchical document outline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentHierarchyNode {
    pub id: NodeId,
    pub level: HierarchyLevel,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub numbering_scheme: Option<NumberingScheme>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub numbering_scope: Option<NumberingScope>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section_id: Option<NodeId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_anchor: Option<TextAnchor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_paragraph_id: Option<NodeId>,
    #[serde(default)]
    pub children: Vec<DocumentHierarchyNode>,
}

impl DocumentHierarchyNode {
    pub fn new(level: HierarchyLevel, title: impl Into<String>) -> Self {
        Self {
            id: NodeId::new(),
            level,
            title: title.into(),
            number_label: None,
            numbering_scheme: None,
            numbering_scope: None,
            section_id: None,
            start_anchor: None,
            target_paragraph_id: None,
            children: Vec::new(),
        }
    }

    pub fn with_number_label(mut self, label: impl Into<String>) -> Self {
        self.number_label = Some(label.into());
        self
    }

    pub fn with_numbering_scheme(mut self, scheme: NumberingScheme) -> Self {
        self.numbering_scheme = Some(scheme);
        self
    }

    pub fn with_numbering_scope(mut self, scope: NumberingScope) -> Self {
        self.numbering_scope = Some(scope);
        self
    }

    pub fn with_section_id(mut self, section_id: NodeId) -> Self {
        self.section_id = Some(section_id);
        self
    }

    pub fn with_start_anchor(mut self, anchor: TextAnchor) -> Self {
        self.start_anchor = Some(anchor);
        self
    }

    pub fn with_target_paragraph(mut self, paragraph_id: NodeId) -> Self {
        self.target_paragraph_id = Some(paragraph_id);
        self
    }

    pub fn add_child(&mut self, child: DocumentHierarchyNode) {
        self.children.push(child);
    }

    /// Full display title including number label if present (e.g., "סימן א׳: הלכות השכמת הבוקר").
    pub fn full_display_title(&self) -> String {
        if let Some(lbl) = &self.number_label {
            if self.title.trim().is_empty() {
                lbl.clone()
            } else if self.title.starts_with(lbl) {
                self.title.clone()
            } else {
                format!("{}: {}", lbl, self.title)
            }
        } else {
            self.title.clone()
        }
    }
}

/// Rule defining how headings at a specific hierarchy level should be numbered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LevelNumberingRule {
    pub level: HierarchyLevel,
    pub scheme: NumberingScheme,
    #[serde(default)]
    pub scope: NumberingScope,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>,
}

/// Dynamic automatic numbering configuration for the document outline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutlineNumberingConfig {
    #[serde(default)]
    pub level_rules: Vec<LevelNumberingRule>,
    #[serde(default)]
    pub punctuation: GematriaPunctuation,
}

impl Default for OutlineNumberingConfig {
    fn default() -> Self {
        Self::torah_default(GematriaPunctuation::StandardPunctuation)
    }
}

impl OutlineNumberingConfig {
    /// Creates the standard Torah book numbering configuration:
    /// - Part -> "חלק א׳" (Continuous across book)
    /// - Chapter -> "סימן א׳" (Continuous across book)
    /// - Section -> "סעיף א׳" (Resets per section/siman)
    /// - Subsection -> "אות א׳" (Resets per subsection/seif)
    pub fn torah_default(punctuation: GematriaPunctuation) -> Self {
        Self {
            level_rules: vec![
                LevelNumberingRule {
                    level: HierarchyLevel::Part,
                    scheme: NumberingScheme::Custom {
                        prefix: "חלק ".to_string(),
                        base: Box::new(NumberingScheme::HebrewGematria(punctuation)),
                        suffix: String::new(),
                    },
                    scope: NumberingScope::ContinuousBook,
                    prefix: None,
                    suffix: None,
                },
                LevelNumberingRule {
                    level: HierarchyLevel::Chapter,
                    scheme: NumberingScheme::TorahSiman(punctuation),
                    scope: NumberingScope::ContinuousBook,
                    prefix: None,
                    suffix: None,
                },
                LevelNumberingRule {
                    level: HierarchyLevel::Section,
                    scheme: NumberingScheme::TorahSeif(punctuation),
                    scope: NumberingScope::PerSection,
                    prefix: None,
                    suffix: None,
                },
                LevelNumberingRule {
                    level: HierarchyLevel::Subsection,
                    scheme: NumberingScheme::TorahOt(punctuation),
                    scope: NumberingScope::PerSection,
                    prefix: None,
                    suffix: None,
                },
            ],
            punctuation,
        }
    }

    pub fn scheme_for_level(&self, level: &HierarchyLevel) -> Option<NumberingScheme> {
        self.level_rules
            .iter()
            .find(|r| &r.level == level)
            .map(|r| r.scheme.clone())
    }

    pub fn scope_for_level(&self, level: &HierarchyLevel) -> NumberingScope {
        self.level_rules
            .iter()
            .find(|r| &r.level == level)
            .map(|r| r.scope)
            .unwrap_or(NumberingScope::PerSection)
    }
}

/// Complete hierarchical outline and Table of Contents (TOC) structure of a book.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct DocumentOutline {
    #[serde(default)]
    pub root_nodes: Vec<DocumentHierarchyNode>,
}

impl DocumentOutline {
    pub fn new() -> Self {
        Self {
            root_nodes: Vec::new(),
        }
    }

    pub fn add_root_node(&mut self, node: DocumentHierarchyNode) {
        self.root_nodes.push(node);
    }

    /// Finds a node by its unique NodeId anywhere in the hierarchy.
    pub fn find_node(&self, id: NodeId) -> Option<&DocumentHierarchyNode> {
        fn search(nodes: &[DocumentHierarchyNode], id: NodeId) -> Option<&DocumentHierarchyNode> {
            for node in nodes {
                if node.id == id {
                    return Some(node);
                }
                if let Some(found) = search(&node.children, id) {
                    return Some(found);
                }
            }
            None
        }
        search(&self.root_nodes, id)
    }

    /// Finds a mutable reference to a node by its NodeId.
    pub fn find_node_mut(&mut self, id: NodeId) -> Option<&mut DocumentHierarchyNode> {
        fn search(
            nodes: &mut [DocumentHierarchyNode],
            id: NodeId,
        ) -> Option<&mut DocumentHierarchyNode> {
            for node in nodes {
                if node.id == id {
                    return Some(node);
                }
                if let Some(found) = search(&mut node.children, id) {
                    return Some(found);
                }
            }
            None
        }
        search(&mut self.root_nodes, id)
    }

    /// Collects all nodes in pre-order depth-first traversal with their nesting depth.
    pub fn flatten_in_order(&self) -> Vec<(&DocumentHierarchyNode, usize)> {
        let mut list = Vec::new();
        fn traverse<'a>(
            nodes: &'a [DocumentHierarchyNode],
            depth: usize,
            out: &mut Vec<(&'a DocumentHierarchyNode, usize)>,
        ) {
            for node in nodes {
                out.push((node, depth));
                traverse(&node.children, depth + 1, out);
            }
        }
        traverse(&self.root_nodes, 0, &mut list);
        list
    }

    /// Finds the ancestor path (from root to parent) for a given node id.
    pub fn ancestor_path(&self, target_id: NodeId) -> Option<Vec<&DocumentHierarchyNode>> {
        fn find<'a>(
            nodes: &'a [DocumentHierarchyNode],
            target_id: NodeId,
            current_path: &mut Vec<&'a DocumentHierarchyNode>,
        ) -> bool {
            for node in nodes {
                current_path.push(node);
                if node.id == target_id {
                    return true;
                }
                if find(&node.children, target_id, current_path) {
                    return true;
                }
                current_path.pop();
            }
            false
        }

        let mut path = Vec::new();
        if find(&self.root_nodes, target_id, &mut path) {
            path.pop(); // Remove the target itself, keeping ancestors
            Some(path)
        } else {
            None
        }
    }

    /// Checks if a node is a descendant of another node in the outline hierarchy.
    pub fn is_descendant_of(&self, descendant_id: NodeId, ancestor_id: NodeId) -> bool {
        if descendant_id == ancestor_id {
            return false;
        }
        if let Some(ancestors) = self.ancestor_path(descendant_id) {
            ancestors.iter().any(|a| a.id == ancestor_id)
        } else {
            false
        }
    }

    /// Recalculates number labels for all nodes in the outline tree according to config rules.
    pub fn recalculate_numbers(&mut self, config: &OutlineNumberingConfig) {
        let mut global_counters: HashMap<HierarchyLevel, usize> = HashMap::new();
        Self::recalculate_slice(&mut self.root_nodes, config, &mut global_counters);
    }

    fn recalculate_slice(
        nodes: &mut [DocumentHierarchyNode],
        config: &OutlineNumberingConfig,
        global_counters: &mut HashMap<HierarchyLevel, usize>,
    ) {
        let mut local_counters: HashMap<HierarchyLevel, usize> = HashMap::new();

        for node in nodes.iter_mut() {
            let scheme_opt = node
                .numbering_scheme
                .as_ref()
                .cloned()
                .or_else(|| config.scheme_for_level(&node.level));
            let scope = node
                .numbering_scope
                .unwrap_or_else(|| config.scope_for_level(&node.level));

            if let Some(scheme) = scheme_opt {
                let counter = match scope {
                    NumberingScope::ContinuousBook => {
                        let c = global_counters.entry(node.level.clone()).or_insert(0);
                        *c += 1;
                        *c
                    }
                    NumberingScope::PerSection | NumberingScope::PerPage => {
                        let c = local_counters.entry(node.level.clone()).or_insert(0);
                        *c += 1;
                        *c
                    }
                };

                let formatted = NumberingFormatter::format(
                    &scheme.with_punctuation(config.punctuation),
                    counter,
                );
                node.number_label = Some(formatted);
            }

            Self::recalculate_slice(&mut node.children, config, global_counters);
        }
    }

    /// Inserts a node into the outline and optionally recalculates all number labels.
    pub fn insert_node(
        &mut self,
        parent_id: Option<NodeId>,
        target_index: usize,
        node: DocumentHierarchyNode,
        config: Option<&OutlineNumberingConfig>,
    ) {
        if let Some(pid) = parent_id {
            if let Some(parent) = self.find_node_mut(pid) {
                let idx = target_index.min(parent.children.len());
                parent.children.insert(idx, node);
            } else {
                let idx = target_index.min(self.root_nodes.len());
                self.root_nodes.insert(idx, node);
            }
        } else {
            let idx = target_index.min(self.root_nodes.len());
            self.root_nodes.insert(idx, node);
        }

        if let Some(cfg) = config {
            self.recalculate_numbers(cfg);
        }
    }

    /// Removes a node by ID and optionally recalculates all number labels.
    pub fn remove_node(
        &mut self,
        node_id: NodeId,
        config: Option<&OutlineNumberingConfig>,
    ) -> Option<DocumentHierarchyNode> {
        fn remove_from_slice(
            nodes: &mut Vec<DocumentHierarchyNode>,
            target_id: NodeId,
        ) -> Option<DocumentHierarchyNode> {
            if let Some(pos) = nodes.iter().position(|n| n.id == target_id) {
                return Some(nodes.remove(pos));
            }
            for n in nodes.iter_mut() {
                if let Some(removed) = remove_from_slice(&mut n.children, target_id) {
                    return Some(removed);
                }
            }
            None
        }

        let removed = remove_from_slice(&mut self.root_nodes, node_id);
        if removed.is_some() {
            if let Some(cfg) = config {
                self.recalculate_numbers(cfg);
            }
        }
        removed
    }

    /// Reorders a node to a new parent and/or index, dynamically recalculating numbering.
    pub fn reorder_node(
        &mut self,
        node_id: NodeId,
        new_parent_id: Option<NodeId>,
        new_index: usize,
        config: Option<&OutlineNumberingConfig>,
    ) -> bool {
        if let Some(new_pid) = new_parent_id {
            if new_pid == node_id || self.is_descendant_of(new_pid, node_id) {
                return false;
            }
        }

        if let Some(node) = self.remove_node(node_id, None) {
            self.insert_node(new_parent_id, new_index, node, config);
            true
        } else {
            false
        }
    }

    /// Sets custom numbering scheme and scope for a specific node, updating numbering.
    pub fn set_node_numbering(
        &mut self,
        node_id: NodeId,
        scheme: Option<NumberingScheme>,
        scope: Option<NumberingScope>,
        config: Option<&OutlineNumberingConfig>,
    ) -> bool {
        if let Some(node) = self.find_node_mut(node_id) {
            node.numbering_scheme = scheme;
            node.numbering_scope = scope;
            if let Some(cfg) = config {
                self.recalculate_numbers(cfg);
            }
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_outline_construction_and_flattening() {
        let mut outline = DocumentOutline::new();

        let mut part1 = DocumentHierarchyNode::new(HierarchyLevel::Part, "חלק ראשון: אורח חיים")
            .with_number_label("חלק א׳");
        let mut chap1 = DocumentHierarchyNode::new(HierarchyLevel::Chapter, "הלכות השכמת הבוקר")
            .with_number_label("סימן א׳");
        let sec1 = DocumentHierarchyNode::new(HierarchyLevel::Section, "יתגבר כארי לעמוד בבוקר")
            .with_number_label("סעיף א׳");
        let sec2 = DocumentHierarchyNode::new(HierarchyLevel::Section, "שויתי ה׳ לנגדי תמיד")
            .with_number_label("סעיף ב׳");

        let chap1_id = chap1.id;
        let sec2_id = sec2.id;

        chap1.add_child(sec1);
        chap1.add_child(sec2);
        part1.add_child(chap1);
        outline.add_root_node(part1);

        assert!(outline.find_node(chap1_id).is_some());
        assert!(outline.find_node(sec2_id).is_some());

        let flattened = outline.flatten_in_order();
        assert_eq!(flattened.len(), 4);
        assert_eq!(flattened[0].0.title, "חלק ראשון: אורח חיים");
        assert_eq!(flattened[0].1, 0);
        assert_eq!(flattened[1].0.title, "הלכות השכמת הבוקר");
        assert_eq!(flattened[1].1, 1);
        assert_eq!(flattened[2].0.title, "יתגבר כארי לעמוד בבוקר");
        assert_eq!(flattened[2].1, 2);
        assert_eq!(flattened[3].0.title, "שויתי ה׳ לנגדי תמיד");
        assert_eq!(flattened[3].1, 2);

        let ancestors = outline.ancestor_path(sec2_id).expect("Should find path");
        assert_eq!(ancestors.len(), 2);
        assert_eq!(ancestors[0].title, "חלק ראשון: אורח חיים");
        assert_eq!(ancestors[1].title, "הלכות השכמת הבוקר");
    }

    #[test]
    fn test_outline_serialization_roundtrip() {
        let mut outline = DocumentOutline::new();
        let node = DocumentHierarchyNode::new(HierarchyLevel::Chapter, "פרק ראשון")
            .with_number_label("א׳");
        outline.add_root_node(node);

        let json = serde_json::to_string(&outline).unwrap();
        let deserialized: DocumentOutline = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.root_nodes.len(), 1);
        assert_eq!(deserialized.root_nodes[0].title, "פרק ראשון");
    }

    #[test]
    fn test_torah_siman_seif_automatic_numbering() {
        let mut outline = DocumentOutline::new();

        let mut siman1 = DocumentHierarchyNode::new(HierarchyLevel::Chapter, "הלכות השכמת הבוקר");
        let seif1_1 = DocumentHierarchyNode::new(HierarchyLevel::Section, "יתגבר כארי");
        let seif1_2 = DocumentHierarchyNode::new(HierarchyLevel::Section, "שויתי ה׳ לנגדי תמיד");
        siman1.add_child(seif1_1);
        siman1.add_child(seif1_2);

        let mut siman2 = DocumentHierarchyNode::new(HierarchyLevel::Chapter, "הלכות לבישת בגדים");
        let seif2_1 = DocumentHierarchyNode::new(HierarchyLevel::Section, "אל ילבש חלוקו מיושב");
        let seif2_2 = DocumentHierarchyNode::new(HierarchyLevel::Section, "ידקדק בחלוקו");
        let seif2_3 = DocumentHierarchyNode::new(HierarchyLevel::Section, "ינעול מנעל ימין");
        siman2.add_child(seif2_1);
        siman2.add_child(seif2_2);
        siman2.add_child(seif2_3);

        outline.add_root_node(siman1);
        outline.add_root_node(siman2);

        let config =
            OutlineNumberingConfig::torah_default(GematriaPunctuation::StandardPunctuation);
        outline.recalculate_numbers(&config);

        assert_eq!(
            outline.root_nodes[0].number_label.as_deref(),
            Some("סימן א׳")
        );
        assert_eq!(
            outline.root_nodes[0].children[0].number_label.as_deref(),
            Some("סעיף א׳")
        );
        assert_eq!(
            outline.root_nodes[0].children[1].number_label.as_deref(),
            Some("סעיף ב׳")
        );

        assert_eq!(
            outline.root_nodes[1].number_label.as_deref(),
            Some("סימן ב׳")
        );
        assert_eq!(
            outline.root_nodes[1].children[0].number_label.as_deref(),
            Some("סעיף א׳")
        );
        assert_eq!(
            outline.root_nodes[1].children[1].number_label.as_deref(),
            Some("סעיף ב׳")
        );
        assert_eq!(
            outline.root_nodes[1].children[2].number_label.as_deref(),
            Some("סעיף ג׳")
        );

        assert_eq!(
            outline.root_nodes[0].full_display_title(),
            "סימן א׳: הלכות השכמת הבוקר"
        );
    }

    #[test]
    fn test_dynamic_reordering_updates_numbering() {
        let mut outline = DocumentOutline::new();

        let mut siman1 = DocumentHierarchyNode::new(HierarchyLevel::Chapter, "ראשון");
        siman1.add_child(DocumentHierarchyNode::new(
            HierarchyLevel::Section,
            "סעיף מקורי א",
        ));
        siman1.add_child(DocumentHierarchyNode::new(
            HierarchyLevel::Section,
            "סעיף מקורי ב",
        ));

        let siman2 = DocumentHierarchyNode::new(HierarchyLevel::Chapter, "שני");

        let siman2_id = siman2.id;
        outline.add_root_node(siman1);
        outline.add_root_node(siman2);

        let config =
            OutlineNumberingConfig::torah_default(GematriaPunctuation::StandardPunctuation);
        outline.recalculate_numbers(&config);

        assert_eq!(outline.root_nodes[0].title, "ראשון");
        assert_eq!(
            outline.root_nodes[0].number_label.as_deref(),
            Some("סימן א׳")
        );
        assert_eq!(outline.root_nodes[1].title, "שני");
        assert_eq!(
            outline.root_nodes[1].number_label.as_deref(),
            Some("סימן ב׳")
        );

        // Reorder siman2 to become first root item
        let success = outline.reorder_node(siman2_id, None, 0, Some(&config));
        assert!(success);

        assert_eq!(outline.root_nodes[0].title, "שני");
        assert_eq!(
            outline.root_nodes[0].number_label.as_deref(),
            Some("סימן א׳")
        );
        assert_eq!(outline.root_nodes[1].title, "ראשון");
        assert_eq!(
            outline.root_nodes[1].number_label.as_deref(),
            Some("סימן ב׳")
        );
    }

    #[test]
    fn test_insertion_and_deletion_updates_numbering() {
        let mut outline = DocumentOutline::new();
        let mut siman = DocumentHierarchyNode::new(HierarchyLevel::Chapter, "הלכות ציצית");
        let sec1 = DocumentHierarchyNode::new(HierarchyLevel::Section, "סעיף א");
        let sec2 = DocumentHierarchyNode::new(HierarchyLevel::Section, "סעיף ב");
        let sec1_id = sec1.id;
        siman.add_child(sec1);
        siman.add_child(sec2);
        let siman_id = siman.id;
        outline.add_root_node(siman);

        let config =
            OutlineNumberingConfig::torah_default(GematriaPunctuation::StandardPunctuation);
        outline.recalculate_numbers(&config);

        assert_eq!(
            outline.root_nodes[0].children[0].number_label.as_deref(),
            Some("סעיף א׳")
        );
        assert_eq!(
            outline.root_nodes[0].children[1].number_label.as_deref(),
            Some("סעיף ב׳")
        );

        // Insert new seif in between
        let new_sec = DocumentHierarchyNode::new(HierarchyLevel::Section, "סעיף חדש ביניים");
        outline.insert_node(Some(siman_id), 1, new_sec, Some(&config));

        assert_eq!(outline.root_nodes[0].children.len(), 3);
        assert_eq!(
            outline.root_nodes[0].children[0].number_label.as_deref(),
            Some("סעיף א׳")
        );
        assert_eq!(
            outline.root_nodes[0].children[1].number_label.as_deref(),
            Some("סעיף ב׳")
        );
        assert_eq!(
            outline.root_nodes[0].children[2].number_label.as_deref(),
            Some("סעיף ג׳")
        );

        // Delete the original first seif
        let removed = outline.remove_node(sec1_id, Some(&config));
        assert!(removed.is_some());
        assert_eq!(outline.root_nodes[0].children.len(), 2);
        assert_eq!(
            outline.root_nodes[0].children[0].number_label.as_deref(),
            Some("סעיף א׳")
        );
        assert_eq!(
            outline.root_nodes[0].children[1].number_label.as_deref(),
            Some("סעיף ב׳")
        );
    }

    #[test]
    fn test_plain_letters_punctuation_mode() {
        let mut outline = DocumentOutline::new();
        let mut siman = DocumentHierarchyNode::new(HierarchyLevel::Chapter, "הלכות שבת");
        let seif = DocumentHierarchyNode::new(HierarchyLevel::Section, "כלל ראשון");
        siman.add_child(seif);
        outline.add_root_node(siman);

        let config = OutlineNumberingConfig::torah_default(GematriaPunctuation::PlainLetters);
        outline.recalculate_numbers(&config);

        assert_eq!(
            outline.root_nodes[0].number_label.as_deref(),
            Some("סימן א")
        );
        assert_eq!(
            outline.root_nodes[0].children[0].number_label.as_deref(),
            Some("סעיף א")
        );
    }

    #[test]
    fn test_is_descendant_cycle_prevention() {
        let mut outline = DocumentOutline::new();
        let mut siman = DocumentHierarchyNode::new(HierarchyLevel::Chapter, "סימן");
        let seif = DocumentHierarchyNode::new(HierarchyLevel::Section, "סעיף");
        let siman_id = siman.id;
        let seif_id = seif.id;
        siman.add_child(seif);
        outline.add_root_node(siman);

        assert!(outline.is_descendant_of(seif_id, siman_id));
        assert!(!outline.is_descendant_of(siman_id, seif_id));

        // Moving siman inside its own child seif must fail
        let config =
            OutlineNumberingConfig::torah_default(GematriaPunctuation::StandardPunctuation);
        let ok = outline.reorder_node(siman_id, Some(seif_id), 0, Some(&config));
        assert!(!ok, "Cycle creation must be strictly rejected");
    }
}
