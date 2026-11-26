//! Continuous & Hierarchical Table of Contents (TOC) Layout Engine (נושא 4.3).
//!
//! Supports:
//! 1. **Line-by-Line Hierarchical TOC** with dot leaders (`...`) and level indentation.
//! 2. **Continuous Inline / Run-in TOC** flowing entries as a justified Knuth-Plass
//!    paragraph separated by Torah ornaments (`◆`, `❖`, `•`), saving pages in sacred books.
//! 3. **Dynamic Page Reference Resolution** (`DynamicPageRefResolver`) mapping `DocumentOutline`
//!    and heading `ParagraphNode`s to live page numbers and gematria.

use crate::engine::TypesettingEngine;
use crate::gematria::{GematriaEngine, HEBREW_GERSHAYIM};
use crate::geometry::{LineBox, PageLayoutBox, PhysicalRect, TextFrameBox};
use crate::knuth_plass::MeasureProfile;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tok_core::id::{FractionalIndex, NodeId};
use tok_core::model::{DocumentRoot, ParagraphNode};
use tok_core::styles::{ParagraphStyle, Progression, TextAlignment};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TocLayoutMode {
    LineByLine,
    InlineRunIn,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TocConfig {
    pub title: String,
    pub mode: TocLayoutMode,
    pub max_level: u8,
    pub use_gematria_pages: bool,
    pub dot_leader_char: char,
    pub inline_separator: String,
    pub level_indent_pt: f32,
    pub font_family: String,
    pub font_size_pt: f32,
    pub line_height_pt: f32,
}

impl Default for TocConfig {
    fn default() -> Self {
        Self {
            title: "תוכן העניינים".to_string(),
            mode: TocLayoutMode::LineByLine,
            max_level: 3,
            use_gematria_pages: true,
            dot_leader_char: '.',
            inline_separator: " ◆ ".to_string(),
            level_indent_pt: 14.0,
            font_family: "Frank Ruhl Libre".to_string(),
            font_size_pt: 11.5,
            line_height_pt: 16.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TocEntry {
    pub title: String,
    pub level: u8,
    pub target_node_id: NodeId,
    pub page_number: usize,
    pub page_gematria: String,
}

/// Resolves physical page numbers for heading paragraphs from rendered `PageLayoutBox`es.
pub struct DynamicPageRefResolver;

impl DynamicPageRefResolver {
    /// Scans `pages` and maps every `paragraph_id` to `(1_based_page_number, gematria)`.
    pub fn resolve_from_pages(pages: &[PageLayoutBox]) -> HashMap<NodeId, (usize, String)> {
        let mut map = HashMap::new();
        for page in pages {
            let p_num = page.page_index + 1;
            let gem = if page.page_number_gematria.is_empty() {
                GematriaEngine::to_hebrew_numeral(p_num)
            } else {
                page.page_number_gematria.clone()
            };
            for frame in &page.frames {
                for line in &frame.lines {
                    if let Some(pid) = line.paragraph_id {
                        map.entry(pid).or_insert_with(|| (p_num, gem.clone()));
                    }
                }
            }
        }
        map
    }

    /// Collects [`TocEntry`] items from `doc.outline` (or falls back to heading paragraphs in
    /// `doc.sections`) paired with resolved page numbers.
    pub fn extract_toc_entries(
        doc: &DocumentRoot,
        page_map: &HashMap<NodeId, (usize, String)>,
        max_level: u8,
    ) -> Vec<TocEntry> {
        let mut entries = Vec::new();

        if let Some(outline) = &doc.outline {
            fn walk_nodes(
                nodes: &[tok_core::outline::DocumentHierarchyNode],
                page_map: &HashMap<NodeId, (usize, String)>,
                max_level: u8,
                out: &mut Vec<TocEntry>,
            ) {
                for node in nodes {
                    let lvl = (node.level.default_depth() + 1).min(255) as u8;
                    if lvl <= max_level {
                        let (p_num, gem) = node
                            .target_paragraph_id
                            .and_then(|pid| page_map.get(&pid).cloned())
                            .unwrap_or_else(|| (1, GematriaEngine::to_hebrew_numeral(1)));
                        let target_id = node.target_paragraph_id.unwrap_or(node.id);
                        let title = match &node.number_label {
                            Some(lbl) if !lbl.is_empty() && !node.title.starts_with(lbl) => {
                                format!("{}: {}", lbl, node.title)
                            }
                            _ => node.title.clone(),
                        };
                        out.push(TocEntry {
                            title,
                            level: lvl,
                            target_node_id: target_id,
                            page_number: p_num,
                            page_gematria: gem,
                        });
                    }
                    walk_nodes(&node.children, page_map, max_level, out);
                }
            }
            walk_nodes(&outline.root_nodes, page_map, max_level, &mut entries);
            if !entries.is_empty() {
                return entries;
            }
        }

        // Fallback: scan heading paragraphs directly from sections
        for sec in &doc.sections {
            for flow in &sec.flows {
                for para in &flow.paragraphs {
                    if let Some(hl) = &para.heading_level {
                        let lvl = (hl.default_depth() + 1).min(255) as u8;
                        if lvl <= max_level {
                            let (p_num, gem) = page_map
                                .get(&para.id)
                                .cloned()
                                .unwrap_or_else(|| (1, GematriaEngine::to_hebrew_numeral(1)));
                            entries.push(TocEntry {
                                title: para.full_heading_text(),
                                level: lvl,
                                target_node_id: para.id,
                                page_number: p_num,
                                page_gematria: gem,
                            });
                        }
                    }
                }
            }
        }
        entries
    }
}

pub struct TocLayoutEngine;

impl TocLayoutEngine {
    /// Generates formatted TOC [`PageLayoutBox`]es in either `LineByLine` (with dot leaders)
    /// or `InlineRunIn` (Knuth-Plass continuous paragraph) mode.
    pub fn render_toc_pages(
        engine: &TypesettingEngine,
        doc: &DocumentRoot,
        entries: &[TocEntry],
        config: &TocConfig,
    ) -> Vec<PageLayoutBox> {
        let cfg = engine.config();
        let is_rtl = doc.metadata.progression != Progression::Ltr;
        let content_x = if is_rtl {
            cfg.margin_outer_pt
        } else {
            cfg.margin_inner_pt
        };
        let content_y = cfg.margin_top_pt;
        let content_w = (cfg.page_width_pt - cfg.margin_inner_pt - cfg.margin_outer_pt).max(60.0);
        let content_h = (cfg.page_height_pt - cfg.margin_top_pt - cfg.margin_bottom_pt).max(60.0);

        let mut all_lines: Vec<LineBox> = Vec::new();
        let mut cursor_y = 0.0f32;

        // Title line
        let title_style = ParagraphStyle {
            id: "toc-title".into(),
            name: "TOC Title".into(),
            font_family: config.font_family.clone(),
            font_size_pt: config.font_size_pt * 1.35,
            line_height_pt: config.line_height_pt * 1.4,
            font_weight: 700,
            alignment: TextAlignment::Center,
            ..Default::default()
        };
        let title_para = ParagraphNode::new(FractionalIndex::initial(), "toc-title", &config.title);
        let mut title_lines = engine.typeset_paragraph_shaped_with_style(
            &title_para,
            Some(&title_style),
            &title_style.font_family,
            &MeasureProfile::uniform(content_w),
            title_style.font_size_pt,
            title_style.line_height_pt,
        );
        for l in &mut title_lines {
            cursor_y += l.height;
            l.baseline_y = cursor_y;
            all_lines.push(l.clone());
        }
        cursor_y += config.line_height_pt * 0.5;

        match config.mode {
            TocLayoutMode::LineByLine => {
                let avg_char_w = config.font_size_pt * 0.45;
                let dot_w = (config.font_size_pt * 0.28).max(2.0);
                for entry in entries {
                    let indent = (entry.level.saturating_sub(1) as f32) * config.level_indent_pt;
                    let avail_w = (content_w - indent).max(40.0);
                    let page_str = if config.use_gematria_pages {
                        &entry.page_gematria
                    } else {
                        &entry.page_number.to_string()
                    };
                    let title_est = entry.title.chars().count() as f32 * avg_char_w;
                    let page_est = page_str.chars().count() as f32 * avg_char_w;
                    let leader_w = (avail_w - title_est - page_est - 12.0).max(dot_w * 3.0);
                    let dot_count = ((leader_w / dot_w) as usize).clamp(3, 80);
                    let dots: String =
                        std::iter::repeat_n(config.dot_leader_char, dot_count).collect();
                    let formatted = format!("{} {} {}", entry.title, dots, page_str);

                    cursor_y += config.line_height_pt;
                    all_lines.push(LineBox {
                        line_index: all_lines.len(),
                        paragraph_id: Some(entry.target_node_id),
                        baseline_y: cursor_y,
                        height: config.line_height_pt,
                        width: avail_w,
                        glyphs: Vec::new(),
                        text: formatted,
                        is_rtl,
                        fonts: vec![config.font_family.clone()],
                        x_offset: if is_rtl { 0.0 } else { indent },
                        alignment: if is_rtl {
                            TextAlignment::Right
                        } else {
                            TextAlignment::Left
                        },
                        ..Default::default()
                    });
                }
            }
            TocLayoutMode::InlineRunIn => {
                let joined = entries
                    .iter()
                    .map(|e| {
                        let page_str = if config.use_gematria_pages {
                            e.page_gematria.clone()
                        } else {
                            e.page_number.to_string()
                        };
                        format!("{} ({})", e.title, page_str)
                    })
                    .collect::<Vec<_>>()
                    .join(&config.inline_separator);

                if !joined.is_empty() {
                    let body_style = ParagraphStyle {
                        id: "toc-inline".into(),
                        name: "TOC Inline".into(),
                        font_family: config.font_family.clone(),
                        font_size_pt: config.font_size_pt,
                        line_height_pt: config.line_height_pt,
                        alignment: TextAlignment::Justified,
                        ..Default::default()
                    };
                    let p = ParagraphNode::new(FractionalIndex::initial(), "toc-inline", &joined);
                    let body_lines = engine.typeset_paragraph_shaped_with_style(
                        &p,
                        Some(&body_style),
                        &body_style.font_family,
                        &MeasureProfile::uniform(content_w),
                        body_style.font_size_pt,
                        body_style.line_height_pt,
                    );
                    for mut l in body_lines {
                        cursor_y += l.height;
                        l.line_index = all_lines.len();
                        l.baseline_y = cursor_y;
                        all_lines.push(l);
                    }
                }
            }
        }

        // Paginate `all_lines` into one or more PageLayoutBox pages
        let mut pages = Vec::new();
        let mut current_lines: Vec<LineBox> = Vec::new();
        let mut page_start_y = 0.0f32;

        for mut line in all_lines {
            let rel_y = line.baseline_y - page_start_y;
            if rel_y > content_h && !current_lines.is_empty() {
                let p_idx = pages.len();
                let gem = GematriaEngine::to_hebrew_numeral(p_idx + 1);
                pages.push(PageLayoutBox::new(
                    p_idx,
                    gem,
                    PhysicalRect::new(0.0, 0.0, cfg.page_width_pt, cfg.page_height_pt),
                    vec![TextFrameBox {
                        frame_id: format!("toc-frame-{}", p_idx),
                        flow_id: "toc".to_string(),
                        rect: PhysicalRect::new(content_x, content_y, content_w, content_h),
                        lines: std::mem::take(&mut current_lines),
                    }],
                    None,
                ));
                page_start_y = line.baseline_y - line.height;
            }
            line.baseline_y -= page_start_y;
            line.line_index = current_lines.len();
            current_lines.push(line);
        }

        if !current_lines.is_empty() || pages.is_empty() {
            let p_idx = pages.len();
            let gem = GematriaEngine::to_hebrew_numeral(p_idx + 1);
            pages.push(PageLayoutBox::new(
                p_idx,
                gem,
                PhysicalRect::new(0.0, 0.0, cfg.page_width_pt, cfg.page_height_pt),
                vec![TextFrameBox {
                    frame_id: format!("toc-frame-{}", p_idx),
                    flow_id: "toc".to_string(),
                    rect: PhysicalRect::new(content_x, content_y, content_w, content_h),
                    lines: current_lines,
                }],
                None,
            ));
        }

        let _ = HEBREW_GERSHAYIM;
        pages
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::TypesettingEngineConfig;
    use tok_core::model::SectionNode;
    use tok_core::outline::HierarchyLevel;

    #[test]
    fn test_toc_line_by_line_dot_leaders_and_inline_run_in() {
        let mut doc = DocumentRoot::new("ספר הלכה");
        let mut sec = SectionNode::new("שער א", "default");
        let h1 = ParagraphNode::new(
            FractionalIndex::initial(),
            "default-heading",
            "סימן א: דיני תפילה",
        )
        .with_heading_level(HierarchyLevel::Chapter);
        let h2 = ParagraphNode::new(
            FractionalIndex::initial(),
            "default-heading",
            "סימן ב: דיני ברכות",
        )
        .with_heading_level(HierarchyLevel::Chapter);
        let h1_id = h1.id;
        let h2_id = h2.id;
        sec.main_flow_mut().unwrap().paragraphs = vec![h1, h2];
        doc.sections = vec![sec];

        let engine = TypesettingEngine::new(TypesettingEngineConfig::default());
        let pages = engine.typeset_document(&doc);
        let page_map = DynamicPageRefResolver::resolve_from_pages(&pages);
        assert!(page_map.contains_key(&h1_id));
        assert!(page_map.contains_key(&h2_id));

        let entries = DynamicPageRefResolver::extract_toc_entries(&doc, &page_map, 3);
        assert_eq!(entries.len(), 2);

        // 1. Line-by-line with dot leaders
        let toc_lines_pages = TocLayoutEngine::render_toc_pages(
            &engine,
            &doc,
            &entries,
            &TocConfig {
                mode: TocLayoutMode::LineByLine,
                ..Default::default()
            },
        );
        assert!(!toc_lines_pages.is_empty());
        let frame_lines = &toc_lines_pages[0].frames[0].lines;
        assert!(frame_lines.iter().any(|l| l.text.contains("...")));

        // 2. Inline / Run-in Knuth-Plass TOC
        let toc_inline_pages = TocLayoutEngine::render_toc_pages(
            &engine,
            &doc,
            &entries,
            &TocConfig {
                mode: TocLayoutMode::InlineRunIn,
                ..Default::default()
            },
        );
        assert!(!toc_inline_pages.is_empty());
        let combined: String = toc_inline_pages[0].frames[0]
            .lines
            .iter()
            .map(|l| l.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(combined.contains("◆"));
    }
}
