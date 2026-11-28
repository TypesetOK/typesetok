//! Hebrew script-mode heuristic classifier (Square / Meruba vs Semi-Cursive Rashi).
//!
//! Inspired by HSMC (Hebrew Script Mode Classifier): uses font-name metadata when
//! available from vector PDFs, or connected-component aspect-ratio/density heuristics
//! on raster scans to distinguish Square body text from Rashi commentary.

use crate::components::ComponentStats;
use crate::profile::DetectedRegion;
use crate::typography::ScriptMode;

/// Classifies a single region's script mode from vector font name or raster component stats.
pub fn classify_region_script(
    region: &DetectedRegion,
    raster_stats: Option<&ComponentStats>,
) -> ScriptMode {
    if let Some(is_rashi) = region.is_rashi_script {
        return if is_rashi {
            ScriptMode::Rashi
        } else {
            ScriptMode::Square
        };
    }

    if let Some(font) = &region.font_name {
        let lower = font.to_lowercase();
        if lower.contains("rashi") || lower.contains("shlomo") || lower.contains("solitreo") {
            return ScriptMode::Rashi;
        }
        if lower.contains("frank")
            || lower.contains("vilna")
            || lower.contains("david")
            || lower.contains("hadas")
            || lower.contains("ashurit")
            || lower.contains("taamey")
        {
            return ScriptMode::Square;
        }
    }

    if let Some(stats) = raster_stats {
        if stats.count >= 25 {
            // Rashi script typically exhibits narrower/taller semi-cursive strokes and higher
            // stroke fragmentation than uniform Square Ashurit glyphs.
            if stats.median_aspect_ratio < 0.72 && stats.ink_density < 0.28 {
                return ScriptMode::Rashi;
            }
            return ScriptMode::Square;
        }
    }

    ScriptMode::Unknown
}

/// Aggregates region-level script classifications into a page-level [`ScriptMode`] and
/// checks whether Square (central) vs Rashi (side) contrast is present.
pub fn aggregate_page_script_mode(regions: &[DetectedRegion]) -> (ScriptMode, bool) {
    let mut square_count = 0usize;
    let mut rashi_count = 0usize;

    for r in regions {
        match r.is_rashi_script {
            Some(true) => rashi_count += 1,
            Some(false) => square_count += 1,
            None => {}
        }
    }

    let mode = match (square_count > 0, rashi_count > 0) {
        (true, true) => ScriptMode::Mixed,
        (true, false) => ScriptMode::Square,
        (false, true) => ScriptMode::Rashi,
        (false, false) => ScriptMode::Unknown,
    };

    let has_contrast = square_count > 0 && rashi_count > 0;
    (mode, has_contrast)
}
