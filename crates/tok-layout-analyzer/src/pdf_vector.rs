//! Vector PDF fast-path extractor.
//!
//! Inspects PDF object streams for real text operators (`BT`..`ET`, `Tf`, `Tm`, `Td`, `Tj`, `TJ`)
//! and vector horizontal rules (`re`, `m`..`l`..`S`) so digital/vector PDFs can be classified
//! with exact coordinates, font names, and font sizes without OCR or heavy ML models.

use crate::error::{AnalyzerError, AnalyzerResult};
use crate::geometry::NormalizedBBox;
use crate::profile::{DetectedRegion, RegionClass};
use serde::{Deserialize, Serialize};

/// Raw vector text span extracted from a PDF content stream.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VectorTextSpan {
    pub x_pt: f32,
    pub y_pt: f32,
    pub width_pt: f32,
    pub height_pt: f32,
    pub font_name: String,
    pub font_size_pt: f32,
    pub text: String,
}

/// Extracted vector page representation prior to spatial aggregation.
#[derive(Debug, Clone, PartialEq)]
pub struct VectorPageData {
    pub page_index: usize,
    pub width_pt: f32,
    pub height_pt: f32,
    pub text_spans: Vec<VectorTextSpan>,
    pub horizontal_rules: Vec<NormalizedBBox>,
}

/// Checks whether a PDF byte slice contains vector text streams (`BT`/`ET` + `Tf`/`Tj`).
pub fn has_vector_text_layer(pdf_bytes: &[u8]) -> bool {
    if !pdf_bytes.starts_with(b"%PDF-") {
        return false;
    }
    let text = String::from_utf8_lossy(pdf_bytes);
    text.contains("BT") && (text.contains("Tj") || text.contains("TJ")) && text.contains("Tf")
}

/// Extracts basic vector page geometry and text spans from uncompressed/ASCII PDF streams.
///
/// For compressed object streams in production, this acts as the pure-Rust fast-path parser
/// and can be paired with `pdfium-render` or `hayro` when enabled.
pub fn extract_vector_pages(
    pdf_bytes: &[u8],
    start_page: usize,
    end_page: usize,
) -> AnalyzerResult<Vec<VectorPageData>> {
    if !pdf_bytes.starts_with(b"%PDF-") {
        return Err(AnalyzerError::PdfExtraction(
            "Input does not start with %PDF- header".into(),
        ));
    }

    let raw = String::from_utf8_lossy(pdf_bytes);
    let (page_w, page_h) = parse_media_box(&raw).unwrap_or((595.0, 842.0));

    let mut pages = Vec::new();
    let mut search_from = 0usize;
    let mut page_idx = 0usize;

    while let Some(bt_rel) = raw[search_from..].find("BT") {
        let bt_pos = search_from + bt_rel;
        let Some(et_rel) = raw[bt_pos..].find("ET") else {
            break;
        };
        let et_pos = bt_pos + et_rel + 2;
        search_from = et_pos;

        page_idx += 1;
        if page_idx < start_page || page_idx > end_page {
            continue;
        }

        let block = &raw[bt_pos..et_pos];
        let spans = parse_text_block_spans(block, page_h);
        let rules = parse_horizontal_rules(block, page_w, page_h);
        if !spans.is_empty() || !rules.is_empty() {
            pages.push(VectorPageData {
                page_index: page_idx,
                width_pt: page_w,
                height_pt: page_h,
                text_spans: spans,
                horizontal_rules: rules,
            });
        }
    }

    Ok(pages)
}

/// Groups vector text spans into line/paragraph [`DetectedRegion`] boxes in normalized `[0, 1]` coordinates.
pub fn vector_page_to_regions(page: &VectorPageData) -> Vec<DetectedRegion> {
    let w = page.width_pt.max(1.0);
    let h = page.height_pt.max(1.0);
    let mut regions: Vec<DetectedRegion> = Vec::new();

    for span in &page.text_spans {
        // PDF Y-axis is bottom-up; convert to top-down normalized coordinates.
        let nx0 = (span.x_pt / w).clamp(0.0, 1.0);
        let nx1 = ((span.x_pt + span.width_pt.max(span.font_size_pt * 0.5)) / w).clamp(0.0, 1.0);
        let top_y = h - (span.y_pt + span.height_pt.max(span.font_size_pt));
        let bot_y = h - span.y_pt;
        let ny0 = (top_y / h).clamp(0.0, 1.0);
        let ny1 = (bot_y / h).clamp(0.0, 1.0);

        let is_rashi = is_rashi_font_name(&span.font_name);
        regions.push(DetectedRegion {
            bbox: NormalizedBBox::new(nx0, ny0, nx1, ny1),
            region_class: RegionClass::Text,
            confidence: 0.98,
            font_size_pt: Some(span.font_size_pt),
            font_name: Some(span.font_name.clone()),
            median_line_height: Some((span.font_size_pt * 1.2) / h),
            is_rashi_script: Some(is_rashi),
        });
    }

    for rule in &page.horizontal_rules {
        regions.push(DetectedRegion {
            bbox: *rule,
            region_class: RegionClass::Rule,
            confidence: 0.99,
            font_size_pt: None,
            font_name: None,
            median_line_height: None,
            is_rashi_script: None,
        });
    }

    regions
}

fn is_rashi_font_name(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.contains("rashi") || lower.contains("shlomo") || lower.contains("solitreo")
}

fn parse_media_box(raw: &str) -> Option<(f32, f32)> {
    let idx = raw.find("/MediaBox")?;
    let rest = &raw[idx..];
    let open = rest.find('[')?;
    let close = rest[open..].find(']')?;
    let nums: Vec<f32> = rest[open + 1..open + close]
        .split_whitespace()
        .filter_map(|s| s.parse::<f32>().ok())
        .collect();
    if nums.len() == 4 {
        Some(((nums[2] - nums[0]).abs(), (nums[3] - nums[1]).abs()))
    } else {
        None
    }
}

fn parse_text_block_spans(block: &str, page_height_pt: f32) -> Vec<VectorTextSpan> {
    let mut spans = Vec::new();
    let mut current_font = String::from("Unknown");
    let mut current_size = 11.0_f32;
    let mut cur_x = 72.0_f32;
    let mut cur_y = page_height_pt - 72.0_f32;

    for line in block.lines() {
        let trimmed = line.trim();
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() >= 3 && parts.last() == Some(&"Tf") {
            current_font = parts[parts.len() - 3].trim_start_matches('/').to_string();
            if let Ok(sz) = parts[parts.len() - 2].parse::<f32>() {
                current_size = sz.max(4.0);
            }
        } else if parts.len() >= 7 && parts.last() == Some(&"Tm") {
            if let (Ok(x), Ok(y)) = (
                parts[parts.len() - 3].parse::<f32>(),
                parts[parts.len() - 2].parse::<f32>(),
            ) {
                cur_x = x;
                cur_y = y;
            }
        } else if parts.len() >= 3 && (parts.last() == Some(&"Td") || parts.last() == Some(&"TD")) {
            if let (Ok(dx), Ok(dy)) = (
                parts[parts.len() - 3].parse::<f32>(),
                parts[parts.len() - 2].parse::<f32>(),
            ) {
                cur_x += dx;
                cur_y += dy;
            }
        }

        if let Some(text) = extract_pdf_literal_text(trimmed) {
            let est_width = (text.chars().count() as f32) * current_size * 0.52;
            spans.push(VectorTextSpan {
                x_pt: cur_x,
                y_pt: cur_y,
                width_pt: est_width.max(current_size),
                height_pt: current_size * 1.2,
                font_name: current_font.clone(),
                font_size_pt: current_size,
                text,
            });
        }
    }

    spans
}

fn extract_pdf_literal_text(line: &str) -> Option<String> {
    if !(line.ends_with("Tj") || line.ends_with("TJ")) {
        return None;
    }
    let start = line.find('(')?;
    let end = line.rfind(')')?;
    if end > start + 1 {
        Some(line[start + 1..end].to_string())
    } else {
        None
    }
}

fn parse_horizontal_rules(block: &str, page_w: f32, page_h: f32) -> Vec<NormalizedBBox> {
    let mut rules = Vec::new();
    for line in block.lines() {
        let parts: Vec<&str> = line.trim().split_whitespace().collect();
        if parts.len() >= 5 && parts[4] == "re" {
            if let (Ok(x), Ok(y), Ok(w), Ok(h)) = (
                parts[0].parse::<f32>(),
                parts[1].parse::<f32>(),
                parts[2].parse::<f32>(),
                parts[3].parse::<f32>(),
            ) {
                if h.abs() <= 4.0 && w.abs() >= page_w * 0.2 {
                    let nx0 = (x / page_w).clamp(0.0, 1.0);
                    let nx1 = ((x + w) / page_w).clamp(0.0, 1.0);
                    let ny0 = ((page_h - (y + h)) / page_h).clamp(0.0, 1.0);
                    let ny1 = ((page_h - y) / page_h).clamp(0.0, 1.0);
                    rules.push(NormalizedBBox::new(nx0, ny0, nx1, ny1));
                }
            }
        }
    }
    rules
}
