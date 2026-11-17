//! Active Anchor Synchronizer & Vertical Spring Convergence.
//!
//! Solves vertical displacement between central text anchors and commentary headings (דיבור המתחיל).
//! Provides active typographic remedies:
//! - Elastic vertical springs (glue insertion) above commentary paragraphs when commentary leads ($\Delta y < 0$).
//! - Automatic page cut detection when commentary lags excessively ($\Delta y > \text{threshold}$).
//! - Baseline grid snapping to maintain typographic rhythm across columns.

use crate::geometry::LineBox;

/// Unique identifier of a textual anchor (e.g. "dh_meematai", "anchor_p1_w4").
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AnchorKey(pub String);

impl AnchorKey {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

/// A physical point on the page where an anchor or commentary heading is placed.
#[derive(Debug, Clone, PartialEq)]
pub struct AnchorPoint {
    pub key: AnchorKey,
    pub stream_id: String,
    pub paragraph_index: usize,
    pub line_index: usize,
    pub y_pt: f32,
}

/// Active typographic remedy calculated for an anchor pair.
#[derive(Debug, Clone, PartialEq)]
pub enum SyncRemedy {
    /// Perfectly aligned ($\Delta y \approx 0$).
    ExactMatch,
    /// Commentary was premature; an elastic vertical spring of `spring_pt` was inserted
    /// immediately before the commentary paragraph to align its baseline with the anchor.
    VerticalSpringApplied { spring_pt: f32 },
    /// Commentary trails after the anchor within acceptable reading tolerance.
    TolerableLag { lag_pt: f32 },
    /// Commentary lag is too large to solve on this page; the central text must be cut
    /// and pushed to the subsequent page along with this commentary item.
    RequirePageCut { anchor_key: AnchorKey, lag_pt: f32 },
    /// Commentary leads by more than the maximum permissible vertical spring.
    ExcessiveLead { lead_pt: f32 },
}

/// Result of evaluating and actively remedying an anchor pair.
#[derive(Debug, Clone, PartialEq)]
pub struct ActiveSyncResult {
    pub anchor_key: AnchorKey,
    pub gemara_y_pt: f32,
    pub commentary_stream: String,
    pub natural_y_pt: f32,
    pub adjusted_y_pt: f32,
    pub spring_injected_pt: f32,
    pub remedy: SyncRemedy,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SynchronizerConfig {
    /// Maximum vertical spring (pt) that may be inserted above a commentary paragraph.
    pub max_vertical_spring_pt: f32,
    /// Maximum vertical lag (pt) commentary may trail behind central text anchor before requiring a page cut.
    pub max_tolerable_lag_pt: f32,
    /// Optional baseline grid step (pt) for locking baselines across columns.
    pub grid_step_pt: Option<f32>,
}

impl Default for SynchronizerConfig {
    fn default() -> Self {
        Self {
            max_vertical_spring_pt: 120.0,
            max_tolerable_lag_pt: 70.0,
            grid_step_pt: Some(13.5),
        }
    }
}

pub struct ActiveSynchronizer;

impl ActiveSynchronizer {
    /// Evaluates displacement between a central anchor and its commentary heading,
    /// applying active spring remedies and baseline snapping.
    pub fn evaluate_and_remedy(
        config: &SynchronizerConfig,
        anchor_key: &AnchorKey,
        gemara_y_pt: f32,
        commentary_stream: &str,
        natural_commentary_y_pt: f32,
    ) -> ActiveSyncResult {
        let delta_y = natural_commentary_y_pt - gemara_y_pt;

        if delta_y.abs() <= 1.0 {
            // Already aligned within 1 point
            return ActiveSyncResult {
                anchor_key: anchor_key.clone(),
                gemara_y_pt,
                commentary_stream: commentary_stream.to_string(),
                natural_y_pt: natural_commentary_y_pt,
                adjusted_y_pt: natural_commentary_y_pt,
                spring_injected_pt: 0.0,
                remedy: SyncRemedy::ExactMatch,
            };
        }

        if delta_y < 0.0 {
            // Commentary leads (starts earlier than Gemara anchor)
            let lead_pt = -delta_y;
            if lead_pt <= config.max_vertical_spring_pt {
                // Apply active vertical spring: push commentary down to align with anchor
                let mut target_y = gemara_y_pt;
                if let Some(step) = config.grid_step_pt {
                    if step > 0.0 {
                        target_y = (target_y / step).round() * step;
                    }
                }
                let spring_pt = (target_y - natural_commentary_y_pt).max(0.0);

                ActiveSyncResult {
                    anchor_key: anchor_key.clone(),
                    gemara_y_pt,
                    commentary_stream: commentary_stream.to_string(),
                    natural_y_pt: natural_commentary_y_pt,
                    adjusted_y_pt: natural_commentary_y_pt + spring_pt,
                    spring_injected_pt: spring_pt,
                    remedy: SyncRemedy::VerticalSpringApplied { spring_pt },
                }
            } else {
                ActiveSyncResult {
                    anchor_key: anchor_key.clone(),
                    gemara_y_pt,
                    commentary_stream: commentary_stream.to_string(),
                    natural_y_pt: natural_commentary_y_pt,
                    adjusted_y_pt: natural_commentary_y_pt,
                    spring_injected_pt: 0.0,
                    remedy: SyncRemedy::ExcessiveLead { lead_pt },
                }
            }
        } else {
            // Commentary lags (starts after Gemara anchor)
            let lag_pt = delta_y;
            if lag_pt <= config.max_tolerable_lag_pt {
                ActiveSyncResult {
                    anchor_key: anchor_key.clone(),
                    gemara_y_pt,
                    commentary_stream: commentary_stream.to_string(),
                    natural_y_pt: natural_commentary_y_pt,
                    adjusted_y_pt: natural_commentary_y_pt,
                    spring_injected_pt: 0.0,
                    remedy: SyncRemedy::TolerableLag { lag_pt },
                }
            } else {
                ActiveSyncResult {
                    anchor_key: anchor_key.clone(),
                    gemara_y_pt,
                    commentary_stream: commentary_stream.to_string(),
                    natural_y_pt: natural_commentary_y_pt,
                    adjusted_y_pt: natural_commentary_y_pt,
                    spring_injected_pt: 0.0,
                    remedy: SyncRemedy::RequirePageCut {
                        anchor_key: anchor_key.clone(),
                        lag_pt,
                    },
                }
            }
        }
    }

    /// Shifts lines in a commentary paragraph down by an injected vertical spring amount,
    /// updating `baseline_y` without altering contiguous line indices or cluster mappings.
    pub fn apply_spring_offset(lines: &mut [LineBox], spring_pt: f32) {
        if spring_pt <= 0.0 {
            return;
        }
        for line in lines.iter_mut() {
            line.baseline_y += spring_pt;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_active_sync_exact_match() {
        let config = SynchronizerConfig::default();
        let key = AnchorKey::new("dh_1");
        let res = ActiveSynchronizer::evaluate_and_remedy(&config, &key, 100.0, "rashi", 100.5);

        assert_eq!(res.remedy, SyncRemedy::ExactMatch);
        assert_eq!(res.spring_injected_pt, 0.0);
    }

    #[test]
    fn test_active_sync_vertical_spring_insertion_on_lead() {
        let config = SynchronizerConfig {
            grid_step_pt: None, // Disable snapping for exact delta test
            ..Default::default()
        };
        let key = AnchorKey::new("dh_lead");
        // Gemara anchor is at y=150. Commentary naturally arrives at y=100 (50pt premature)
        let res = ActiveSynchronizer::evaluate_and_remedy(&config, &key, 150.0, "rashi", 100.0);

        assert_eq!(
            res.remedy,
            SyncRemedy::VerticalSpringApplied { spring_pt: 50.0 }
        );
        assert_eq!(res.spring_injected_pt, 50.0);
        assert_eq!(res.adjusted_y_pt, 150.0); // Now perfectly aligned with Gemara anchor!
    }

    #[test]
    fn test_active_sync_tolerable_lag() {
        let config = SynchronizerConfig::default();
        let key = AnchorKey::new("dh_lag");
        // Gemara anchor is at y=100. Commentary starts at y=130 (30pt lag <= 70pt max)
        let res = ActiveSynchronizer::evaluate_and_remedy(&config, &key, 100.0, "tosafot", 130.0);

        assert_eq!(res.remedy, SyncRemedy::TolerableLag { lag_pt: 30.0 });
        assert_eq!(res.spring_injected_pt, 0.0);
        assert_eq!(res.adjusted_y_pt, 130.0);
    }

    #[test]
    fn test_active_sync_requires_page_cut_on_excessive_lag() {
        let config = SynchronizerConfig::default();
        let key = AnchorKey::new("dh_overflow_lag");
        // Gemara anchor is at y=100. Commentary delayed until y=220 (120pt lag > 70pt max)
        let res = ActiveSynchronizer::evaluate_and_remedy(&config, &key, 100.0, "rashi", 220.0);

        assert_eq!(
            res.remedy,
            SyncRemedy::RequirePageCut {
                anchor_key: key,
                lag_pt: 120.0,
            }
        );
    }

    #[test]
    fn test_apply_spring_offset_preserves_line_indices() {
        let mut lines = vec![
            LineBox {
                line_index: 0,
                paragraph_id: None,
                baseline_y: 18.0,
                height: 18.0,
                width: 140.0,
                glyphs: Vec::new(),
                text: "line 0".to_string(),
                is_rtl: true,
                fonts: Vec::new(),
                is_drop_cap: false,
                x_offset: 0.0,
                ..Default::default()
            },
            LineBox {
                line_index: 1,
                paragraph_id: None,
                baseline_y: 36.0,
                height: 18.0,
                width: 140.0,
                glyphs: Vec::new(),
                text: "line 1".to_string(),
                is_rtl: true,
                fonts: Vec::new(),
                is_drop_cap: false,
                x_offset: 0.0,
                ..Default::default()
            },
        ];

        ActiveSynchronizer::apply_spring_offset(&mut lines, 40.0);

        assert_eq!(lines[0].line_index, 0);
        assert_eq!(lines[0].baseline_y, 58.0);
        assert_eq!(lines[1].line_index, 1);
        assert_eq!(lines[1].baseline_y, 76.0);
    }
}
