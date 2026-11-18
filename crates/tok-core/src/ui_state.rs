use crate::header::FootnoteApparatusConfig;
use crate::id::{FractionalIndex, NodeId};
use crate::model::{DocumentRoot, Flow, FlowId, FlowType, ParagraphNode, SectionNode};
use crate::multi_flow_anchor::MultiFlowAnchorMap;
use crate::spans::TextSpan;
use crate::styles::GrepRule;
use std::collections::HashMap;

#[derive(serde::Deserialize)]
pub struct RawMultiFlowState {
    pub title: Option<String>,
    #[serde(rename = "templateType")]
    pub template_type: Option<String>,
    #[serde(default)]
    pub flows: HashMap<String, Vec<RawStoryPara>>,
    #[serde(default)]
    pub settings: Option<serde_json::Value>,
    #[serde(default, rename = "multiFlowAnchors")]
    pub multi_flow_anchors: Option<MultiFlowAnchorMap>,
    #[serde(default)]
    pub anchors: Option<MultiFlowAnchorMap>,
    #[serde(default, rename = "footnoteApparatuses")]
    pub footnote_apparatuses: Vec<FootnoteApparatusConfig>,
    #[serde(default, rename = "grepRules")]
    pub grep_rules: Vec<GrepRule>,
}

#[derive(serde::Deserialize)]
pub struct RawStoryPara {
    pub id: Option<String>,
    #[serde(rename = "styleId")]
    pub style_id: Option<String>,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub spans: Vec<TextSpan>,
}

/// Converts a raw UI multi-flow document state into a canonical `DocumentRoot`.
pub fn convert_multi_flow_state_to_root(raw: RawMultiFlowState) -> DocumentRoot {
    let title = raw.title.unwrap_or_else(|| "מסמך ללא שם".to_string());
    let mut root = DocumentRoot::new(&title);
    if let Some(settings) = raw.settings {
        root.metadata.document_settings = Some(settings);
    }
    root.multi_flow_anchors = raw.multi_flow_anchors.or(raw.anchors);
    root.footnote_apparatuses = raw.footnote_apparatuses;
    root.grep_rules = raw.grep_rules;
    root.sections.clear();

    let is_prose = raw.template_type.as_deref() == Some("prose");
    let mut section = SectionNode::new(&title, "chapter-first");
    section.flows.clear();

    for (flow_key, paras) in raw.flows {
        let is_apparatus = root
            .footnote_apparatuses
            .iter()
            .any(|a| a.id == flow_key || a.flow_id.as_deref() == Some(flow_key.as_str()));
        let is_footnote_flow = flow_key == "notes" || flow_key == "footnote" || is_apparatus;
        if is_prose
            && flow_key != "gemara"
            && flow_key != "main"
            && !(is_footnote_flow && !paras.is_empty())
        {
            continue;
        }
        let (flow_id, flow_type) = match flow_key.as_str() {
            "gemara" => {
                if is_prose {
                    (FlowId::main(), FlowType::Main)
                } else {
                    (FlowId::new("gemara"), FlowType::Main)
                }
            }
            "rashi" => (FlowId::new("rashi"), FlowType::CommentA),
            "tosafot" => (FlowId::new("tosafot"), FlowType::CommentB),
            "notes" => (FlowId::new("notes"), FlowType::Footnote),
            "main" => (FlowId::main(), FlowType::Main),
            other if is_apparatus || other.contains("note") => {
                (FlowId::new(other), FlowType::Footnote)
            }
            other => (FlowId::new(other), FlowType::Main),
        };

        let mut flow = Flow::new(flow_id, flow_type);
        let mut prev_idx: Option<FractionalIndex> = None;
        for p in paras {
            let next_idx = match FractionalIndex::between(prev_idx.as_ref(), None) {
                Ok(idx) => idx,
                Err(_) => FractionalIndex::initial(),
            };
            let style = p.style_id.unwrap_or_else(|| "default-body".to_string());
            let mut node = ParagraphNode::new(next_idx.clone(), style, &p.text);
            if !p.spans.is_empty() {
                node = node.with_spans(p.spans);
            }
            if let Some(ref id_str) = p.id {
                if let Ok(node_id) = NodeId::from_string(id_str) {
                    node.id = node_id;
                }
            }
            flow.paragraphs.push(node);
            prev_idx = Some(next_idx);
        }
        section.flows.push(flow);
    }

    if section.flows.is_empty() {
        section
            .flows
            .push(Flow::new(FlowId::main(), FlowType::Main));
    }

    root.sections.push(section);
    root
}
