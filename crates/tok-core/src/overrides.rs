use serde::{Deserialize, Serialize};

/// Target page parity mode for forced page breaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PageBreakMode {
    /// Natural pagination flow without forced page break.
    #[default]
    Auto,
    /// Force break to the next page.
    Page,
    /// Force break to the next odd (recto) page, injecting a blank verso page if needed.
    OddPage,
    /// Force break to the next even (verso) page.
    EvenPage,
}

/// Manual typographic lock overrides specified by the user or editor.
///
/// Ensures that manual editorial interventions (e.g., custom line breaks,
/// locked page breaks, or pinned paragraph positions) are preserved and respected
/// by the automatic typesetting and pagination engine.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct LayoutLockOverrides {
    /// Keep all lines of this paragraph on the same page (no intra-paragraph page splits).
    #[serde(default)]
    pub keep_lines_together: bool,

    /// Force a page break immediately before this paragraph.
    #[serde(default)]
    pub force_page_break_before: bool,

    /// Force a page break immediately after this paragraph.
    #[serde(default)]
    pub force_page_break_after: bool,

    /// Force this paragraph (e.g. Siman / Perek opening) to open on an odd (recto) page,
    /// automatically injecting a blank verso page if preceding content ended on an odd page.
    #[serde(default)]
    pub force_odd_page_break_before: bool,

    /// Fine-grained page break mode before this paragraph.
    #[serde(default)]
    pub page_break_mode: PageBreakMode,

    /// Character offsets in paragraph text where line breaks are manually locked.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub locked_line_breaks: Vec<usize>,

    /// Additional manual baseline shift or vertical nudge in points.
    #[serde(default)]
    pub manual_leading_offset_pt: f32,

    /// When true, the entire layout of this paragraph is frozen and must not be altered
    /// by Knuth-Plass or spacing recalculations.
    #[serde(default)]
    pub is_frozen: bool,
}

impl LayoutLockOverrides {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn freeze() -> Self {
        Self {
            is_frozen: true,
            ..Default::default()
        }
    }

    pub fn page_break_before() -> Self {
        Self {
            force_page_break_before: true,
            page_break_mode: PageBreakMode::Page,
            ..Default::default()
        }
    }

    pub fn odd_page_break_before() -> Self {
        Self {
            force_odd_page_break_before: true,
            page_break_mode: PageBreakMode::OddPage,
            ..Default::default()
        }
    }

    pub fn page_break_after() -> Self {
        Self {
            force_page_break_after: true,
            ..Default::default()
        }
    }

    pub fn keep_together() -> Self {
        Self {
            keep_lines_together: true,
            ..Default::default()
        }
    }

    pub fn with_locked_breaks(mut self, breaks: Vec<usize>) -> Self {
        self.locked_line_breaks = breaks;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_lock_overrides_builder_and_defaults() {
        let default_locks = LayoutLockOverrides::new();
        assert!(!default_locks.is_frozen);
        assert!(!default_locks.force_page_break_before);
        assert!(!default_locks.keep_lines_together);

        let frozen = LayoutLockOverrides::freeze();
        assert!(frozen.is_frozen);

        let pb = LayoutLockOverrides::page_break_before().with_locked_breaks(vec![15, 30]);
        assert!(pb.force_page_break_before);
        assert_eq!(pb.locked_line_breaks, vec![15, 30]);

        let odd_pb = LayoutLockOverrides::odd_page_break_before();
        assert!(odd_pb.force_odd_page_break_before);
        assert_eq!(odd_pb.page_break_mode, PageBreakMode::OddPage);
    }

    #[test]
    fn test_lock_overrides_serde() {
        let locks = LayoutLockOverrides {
            keep_lines_together: true,
            force_page_break_before: true,
            force_page_break_after: false,
            force_odd_page_break_before: true,
            page_break_mode: PageBreakMode::OddPage,
            locked_line_breaks: vec![10, 25],
            manual_leading_offset_pt: 2.5,
            is_frozen: false,
        };

        let json = serde_json::to_string(&locks).unwrap();
        let loaded: LayoutLockOverrides = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded, locks);
    }
}
