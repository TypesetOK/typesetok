//! Inline (Run-in) Notes and Sidenotes (Marginalia) data models and helpers.
//!
//! - **Inline / Run-in Notes (2.2):** Embedded inside paragraph flow as styled inline spans
//!   (e.g., parenthetical commentary or glosses in smaller font) without breaking paragraph flow.
//! - **Sidenotes / Marginalia (2.3):** Margin notes anchored to a paragraph and character offset,
//!   placed in the outer or spine-side margin with vertical collision avoidance (e.g., Mesorat HaShas,
//!   Ein Mishpat).

use crate::id::NodeId;
use crate::normalizer::HebrewNormalizer;
use crate::spans::{insert_range, normalize_spans, TextSpan};
use crate::styles::{CharacterFormat, Color};
use serde::{Deserialize, Serialize};

/// An inline (run-in) note woven into a paragraph's text flow.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InlineNote {
    pub id: NodeId,
    /// Character offset in the base paragraph text where the inline note is inserted.
    pub char_offset: usize,
    /// Normalized note body text.
    pub text: String,
    /// Opening delimiter (default `" ("`).
    #[serde(default = "default_inline_prefix")]
    pub prefix: String,
    /// Closing delimiter (default `")"`).
    #[serde(default = "default_inline_suffix")]
    pub suffix: String,
    /// Font size multiplier relative to the host paragraph's font size (default `0.78`).
    #[serde(default = "default_inline_font_scale")]
    pub font_scale: f32,
    /// Optional override font family (e.g., `"Noto Rashi Hebrew"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,
    #[serde(default)]
    pub italic: bool,
    #[serde(default)]
    pub bold: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
}

fn default_inline_prefix() -> String {
    " (".to_string()
}

fn default_inline_suffix() -> String {
    ")".to_string()
}

fn default_inline_font_scale() -> f32 {
    0.78
}

impl InlineNote {
    pub fn new(char_offset: usize, text: &str) -> Self {
        Self {
            id: NodeId::new(),
            char_offset,
            text: HebrewNormalizer::normalize(text),
            prefix: default_inline_prefix(),
            suffix: default_inline_suffix(),
            font_scale: default_inline_font_scale(),
            font_family: None,
            italic: false,
            bold: false,
            color: None,
        }
    }

    pub fn with_delimiters(mut self, prefix: impl Into<String>, suffix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self.suffix = suffix.into();
        self
    }

    pub fn with_font_scale(mut self, scale: f32) -> Self {
        self.font_scale = scale.clamp(0.3, 1.5);
        self
    }

    pub fn with_font_family(mut self, family: impl Into<String>) -> Self {
        self.font_family = Some(family.into());
        self
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Formats the inline note text with its prefix/suffix and computes its `CharacterFormat`.
    pub fn rendered_text_and_format(&self, base_font_size_pt: f32) -> (String, CharacterFormat) {
        let rendered = format!("{}{}{}", self.prefix, self.text, self.suffix);
        let fmt = CharacterFormat {
            font_family: self.font_family.clone(),
            font_size_pt: Some((base_font_size_pt * self.font_scale).max(4.0)),
            font_weight: if self.bold { Some(700) } else { None },
            italic: if self.italic { Some(true) } else { None },
            color: self.color.clone(),
            ..Default::default()
        };
        (rendered, fmt)
    }
}

/// Non-destructively weaves inline notes into a paragraph's text and span list,
/// shifting existing spans cleanly so `KnuthPlassBreaker` wraps lines around them.
pub fn expand_paragraph_with_inline_notes(
    base_text: &str,
    base_spans: &[TextSpan],
    notes: &[InlineNote],
    base_font_size_pt: f32,
) -> (String, Vec<TextSpan>) {
    if notes.is_empty() {
        return (base_text.to_string(), base_spans.to_vec());
    }

    let mut indexed: Vec<(usize, &InlineNote)> = notes.iter().enumerate().collect();
    indexed.sort_by_key(|(idx, n)| (n.char_offset, *idx));

    let mut text = base_text.to_string();
    let mut spans = base_spans.to_vec();
    let mut delta: usize = 0;

    for (_, note) in indexed {
        let char_len = text.chars().count();
        let insert_char_pos = (note.char_offset + delta).min(char_len);
        let (snippet, fmt) = note.rendered_text_and_format(base_font_size_pt);
        let snippet_chars = snippet.chars().count();
        if snippet_chars == 0 {
            continue;
        }

        let byte_idx = text
            .char_indices()
            .nth(insert_char_pos)
            .map(|(b, _)| b)
            .unwrap_or(text.len());
        text.insert_str(byte_idx, &snippet);

        let inserted_span = vec![TextSpan::new(0, snippet_chars, fmt)];
        spans = insert_range(
            &spans,
            insert_char_pos,
            snippet_chars,
            &inserted_span,
            char_len,
        );
        delta += snippet_chars;
    }

    let final_len = text.chars().count();
    (text, normalize_spans(spans, final_len))
}

/// Which margin a sidenote / marginalia block is placed in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SidenoteMarginSide {
    /// Outer margin (away from spine: left on odd/recto in RTL, right on even/verso in RTL).
    #[default]
    Outer,
    /// Inner / spine-side margin.
    Inner,
    /// Always left margin.
    Left,
    /// Always right margin.
    Right,
}

/// A marginalia / sidenote anchored to a specific paragraph and character offset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sidenote {
    pub id: NodeId,
    /// Character offset within the owning paragraph where this sidenote is anchored.
    pub char_offset: usize,
    /// Optional short label or marker (e.g., `"א"`, `"*"`, `"מסורת הש\"ס"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marker: Option<String>,
    /// Normalized sidenote body text.
    pub text: String,
    /// Optional per-note side override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub side: Option<SidenoteMarginSide>,
    /// Optional width override in points.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width_pt: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_size_pt: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
}

impl Sidenote {
    pub fn new(char_offset: usize, text: &str) -> Self {
        Self {
            id: NodeId::new(),
            char_offset,
            marker: None,
            text: HebrewNormalizer::normalize(text),
            side: None,
            width_pt: None,
            font_family: None,
            font_size_pt: None,
            color: None,
        }
    }

    pub fn with_marker(mut self, marker: impl Into<String>) -> Self {
        self.marker = Some(marker.into());
        self
    }

    pub fn with_side(mut self, side: SidenoteMarginSide) -> Self {
        self.side = Some(side);
        self
    }

    pub fn with_width(mut self, width_pt: f32) -> Self {
        self.width_pt = Some(width_pt.max(18.0));
        self
    }

    pub fn with_font(mut self, family: impl Into<String>, size_pt: f32) -> Self {
        self.font_family = Some(family.into());
        self.font_size_pt = Some(size_pt.max(4.0));
        self
    }
}

/// Section-level configuration for sidenotes & marginalia layout.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SidenoteConfig {
    pub enabled: bool,
    pub default_side: SidenoteMarginSide,
    pub width_pt: f32,
    pub gap_from_body_pt: f32,
    pub vertical_gap_pt: f32,
    pub font_family: String,
    pub font_size_pt: f32,
    pub line_height_ratio: f32,
}

impl Default for SidenoteConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_side: SidenoteMarginSide::Outer,
            width_pt: 72.0,
            gap_from_body_pt: 10.0,
            vertical_gap_pt: 6.0,
            font_family: "Noto Rashi Hebrew".to_string(),
            font_size_pt: 8.5,
            line_height_ratio: 1.25,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_inline_notes_preserves_and_shifts_spans() {
        let base = "אמר רבא הלכתא כוותיה.";
        let base_spans = vec![TextSpan::new(
            0,
            3,
            CharacterFormat {
                font_weight: Some(700),
                ..Default::default()
            },
        )];
        let note = InlineNote::new(8, "עיין תוספות ד\"ה הלכתא");
        let (expanded, spans) =
            expand_paragraph_with_inline_notes(base, &base_spans, &[note], 12.0);
        assert!(expanded.contains(" (עיין תוספות ד\"ה הלכתא)"));
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].start, 0);
        assert_eq!(spans[0].end, 3);
        assert_eq!(spans[0].format.font_weight, Some(700));
        assert_eq!(spans[1].start, 8);
        assert!((spans[1].format.font_size_pt.unwrap() - 12.0 * 0.78).abs() < 0.01);
    }
}
