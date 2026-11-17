use serde::{Deserialize, Serialize};
use std::time::Duration;
use tok_core::id::NodeId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WaiverRecord {
    pub code: String,
    pub reason: String,
    pub approved_by: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutDiagnostic {
    pub code: String,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub page_index: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bottom_delta_pt: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waiver: Option<WaiverRecord>,
}

/// Detailed diagnostic summary of an incremental or full recomputation pass.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecomputeReport {
    pub total_pages_before: usize,
    pub total_pages_after: usize,
    pub first_affected_page: usize,
    pub recomputed_pages_count: usize,
    pub reused_pages_count: usize,
    pub dirty_nodes: Vec<NodeId>,
    pub recomputed_paragraphs_count: usize,
    pub cached_paragraphs_count: usize,
    pub zero_cascade: bool,
    pub convergence_page: Option<usize>,
    pub cache_hit_ratio: f64,
    pub latency_ms: f64,
}

impl RecomputeReport {
    /// Formats a human-readable telemetry summary for diagnostic logs and testing.
    pub fn summary(&self) -> String {
        self.summary_string()
    }

    /// Formats a human-readable telemetry summary for diagnostic logs and testing.
    pub fn summary_string(&self) -> String {
        format!(
            "[RecomputeReport] Pages: {} -> {} (Recomputed: {}, Reused: {} | ZeroCascade: {}, Converged: {:?}) | Paragraphs: Recomputed {}, Cached {} (HitRatio: {:.1}%) | Latency: {:.3}ms",
            self.total_pages_before,
            self.total_pages_after,
            self.recomputed_pages_count,
            self.reused_pages_count,
            self.zero_cascade,
            self.convergence_page,
            self.recomputed_paragraphs_count,
            self.cached_paragraphs_count,
            self.cache_hit_ratio * 100.0,
            self.latency_ms,
        )
    }

    /// Creates a report from IncrementalTypesetDiagnostics and page count before edit.
    pub fn from_diagnostics(
        diag: &crate::incremental::IncrementalTypesetDiagnostics,
        pages_before: usize,
    ) -> Self {
        Self::from_metrics(
            pages_before,
            diag.total_pages,
            diag.first_affected_page,
            diag.recomputed_pages,
            diag.reused_pages,
            Vec::new(),
            diag.recomputed_paragraphs,
            diag.reused_paragraphs,
            diag.zero_cascade,
            diag.converged_at_page,
            diag.duration,
        )
    }

    /// Creates a report from raw typesetting metrics and execution duration.
    #[allow(clippy::too_many_arguments)]
    pub fn from_metrics(
        pages_before: usize,
        pages_after: usize,
        first_affected_page: usize,
        recomputed_pages: usize,
        reused_pages: usize,
        dirty_nodes: Vec<NodeId>,
        recomputed_paras: usize,
        cached_paras: usize,
        zero_cascade: bool,
        convergence_page: Option<usize>,
        duration: Duration,
    ) -> Self {
        let total_paras = recomputed_paras + cached_paras;
        let cache_hit_ratio = if total_paras == 0 {
            0.0
        } else {
            cached_paras as f64 / total_paras as f64
        };

        Self {
            total_pages_before: pages_before,
            total_pages_after: pages_after,
            first_affected_page,
            recomputed_pages_count: recomputed_pages,
            reused_pages_count: reused_pages,
            dirty_nodes,
            recomputed_paragraphs_count: recomputed_paras,
            cached_paragraphs_count: cached_paras,
            zero_cascade,
            convergence_page,
            cache_hit_ratio,
            latency_ms: duration.as_secs_f64() * 1000.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recompute_report_metrics_and_summary() {
        let id = NodeId::new();
        let report = RecomputeReport::from_metrics(
            10,
            10,
            2,
            1,
            9,
            vec![id],
            1,
            49,
            true,
            Some(2),
            Duration::from_micros(850),
        );

        assert_eq!(report.total_pages_before, 10);
        assert_eq!(report.total_pages_after, 10);
        assert!(report.zero_cascade);
        assert_eq!(report.reused_pages_count, 9);
        assert_eq!(report.recomputed_pages_count, 1);
        assert!((report.cache_hit_ratio - 0.98).abs() < 1e-4);
        assert!(report.latency_ms < 1.0);

        let summary = report.summary_string();
        assert!(summary.contains("ZeroCascade: true"));
        assert!(summary.contains("HitRatio: 98.0%"));
    }

    #[test]
    fn test_recompute_report_serde_roundtrip() {
        let report = RecomputeReport::from_metrics(
            5,
            6,
            1,
            5,
            1,
            Vec::new(),
            5,
            20,
            false,
            Some(4),
            Duration::from_millis(12),
        );

        let json = serde_json::to_string(&report).unwrap();
        let loaded: RecomputeReport = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded, report);
    }
}
