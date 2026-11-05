use crate::geometry::LineBox;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tok_core::id::NodeId;
use tok_core::model::ParagraphNode;
use tok_core::styles::ParagraphStyle;

/// Unique cache key for paragraph layout computation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ParagraphCacheKey {
    pub node_id: NodeId,
    pub content_hash: u64,
    pub style_hash: u64,
    pub measure_bits: u32,
}

/// Cached formatted lines and structural metrics for a paragraph.
#[derive(Debug, Clone, PartialEq)]
pub struct CachedParagraphLayout {
    pub lines: Vec<LineBox>,
    pub total_height: f32,
    pub line_count: usize,
}

/// Performance telemetry and hit-rate metrics for paragraph caching.
#[derive(Debug, Default)]
pub struct CacheMetrics {
    pub hits: AtomicUsize,
    pub misses: AtomicUsize,
    pub evictions: AtomicUsize,
}

impl CacheMetrics {
    pub fn hit_count(&self) -> usize {
        self.hits.load(Ordering::Relaxed)
    }

    pub fn miss_count(&self) -> usize {
        self.misses.load(Ordering::Relaxed)
    }

    pub fn eviction_count(&self) -> usize {
        self.evictions.load(Ordering::Relaxed)
    }

    pub fn hit_ratio(&self) -> f64 {
        let h = self.hit_count();
        let m = self.miss_count();
        let total = h + m;
        if total == 0 {
            0.0
        } else {
            h as f64 / total as f64
        }
    }
}

/// Snapshot of cache performance metrics.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CacheMetricsSnapshot {
    pub hits: usize,
    pub misses: usize,
    pub evictions: usize,
    pub hit_ratio: f64,
    pub entry_count: usize,
    pub approx_memory_bytes: usize,
}

/// In-memory cache for paragraph layout results (shaped & justified lines).
///
/// Ensures that unchanged paragraphs are not re-shaped with HarfBuzz or re-broken
/// with Knuth-Plass during incremental editing cascades.
#[derive(Debug, Clone)]
pub struct ParagraphLayoutCache {
    entries: HashMap<NodeId, (ParagraphCacheKey, CachedParagraphLayout)>,
    metrics: Arc<CacheMetrics>,
}

impl Default for ParagraphLayoutCache {
    fn default() -> Self {
        Self::new()
    }
}

impl ParagraphLayoutCache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            metrics: Arc::new(CacheMetrics::default()),
        }
    }

    /// Computes a 64-bit content hash of the paragraph text, patches, and overrides.
    pub fn compute_content_hash(p: &ParagraphNode) -> u64 {
        let mut hasher = DefaultHasher::new();
        p.text.hash(&mut hasher);
        p.style_id.hash(&mut hasher);
        for patch in &p.style_patches {
            patch.font_family.hash(&mut hasher);
            if let Some(sz) = patch.font_size_pt {
                sz.to_bits().hash(&mut hasher);
            }
            if let Some(lh) = patch.line_height_pt {
                lh.to_bits().hash(&mut hasher);
            }
            if let Some(al) = patch.alignment {
                (al as u8).hash(&mut hasher);
            }
        }
        if let Some(overrides) = &p.overrides {
            overrides.keep_lines_together.hash(&mut hasher);
            overrides.force_page_break_before.hash(&mut hasher);
            overrides.force_page_break_after.hash(&mut hasher);
            overrides.locked_line_breaks.hash(&mut hasher);
            overrides
                .manual_leading_offset_pt
                .to_bits()
                .hash(&mut hasher);
            overrides.is_frozen.hash(&mut hasher);
        }
        hasher.finish()
    }

    /// Computes a 64-bit hash of typography style properties affecting line-breaking.
    pub fn compute_style_hash(s: &ParagraphStyle) -> u64 {
        let mut hasher = DefaultHasher::new();
        s.id.hash(&mut hasher);
        s.font_family.hash(&mut hasher);
        s.font_weight.hash(&mut hasher);
        s.font_size_pt.to_bits().hash(&mut hasher);
        s.line_height_pt.to_bits().hash(&mut hasher);
        s.first_line_indent_pt.to_bits().hash(&mut hasher);
        (s.alignment as u8).hash(&mut hasher);
        (s.ahalterm_stretch as u8).hash(&mut hasher);
        hasher.finish()
    }

    /// Builds a cache key for the paragraph with given style and column width.
    pub fn make_key(
        p: &ParagraphNode,
        s: &ParagraphStyle,
        column_width_pt: f32,
    ) -> ParagraphCacheKey {
        // Quantize column width to 0.001pt (1/1000th pt) to eliminate float LSB jitter across solver runs
        let quantized = (column_width_pt * 1000.0).round() as i64;
        ParagraphCacheKey {
            node_id: p.id,
            content_hash: Self::compute_content_hash(p),
            style_hash: Self::compute_style_hash(s),
            measure_bits: quantized as u32,
        }
    }

    /// Looks up cached lines for a paragraph if key matches.
    pub fn get(&self, key: &ParagraphCacheKey) -> Option<&CachedParagraphLayout> {
        if let Some((k, layout)) = self.entries.get(&key.node_id) {
            if k == key {
                self.metrics.hits.fetch_add(1, Ordering::Relaxed);
                return Some(layout);
            }
        }
        self.metrics.misses.fetch_add(1, Ordering::Relaxed);
        None
    }

    /// Inserts or updates the cached layout for a paragraph.
    pub fn insert(&mut self, key: ParagraphCacheKey, lines: Vec<LineBox>) {
        let line_count = lines.len();
        let total_height = lines.iter().map(|l| l.height).sum::<f32>();
        let layout = CachedParagraphLayout {
            lines,
            total_height,
            line_count,
        };
        self.entries.insert(key.node_id, (key, layout));
    }

    /// Invalidates a specific paragraph by NodeId.
    pub fn invalidate(&mut self, node_id: NodeId) -> bool {
        if self.entries.remove(&node_id).is_some() {
            self.metrics.evictions.fetch_add(1, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    /// Clears all cached entries.
    pub fn clear(&mut self) {
        let count = self.entries.len();
        self.entries.clear();
        self.metrics.evictions.fetch_add(count, Ordering::Relaxed);
    }

    /// Number of cached paragraphs.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns reference to cache metrics.
    pub fn metrics(&self) -> &CacheMetrics {
        &self.metrics
    }

    /// Returns approximate memory consumed by cache entries in bytes.
    pub fn approx_memory_bytes(&self) -> usize {
        let entry_base =
            std::mem::size_of::<(NodeId, (ParagraphCacheKey, CachedParagraphLayout))>();
        let mut total = self.entries.len() * entry_base;
        for (_, layout) in self.entries.values() {
            total += layout.lines.len() * std::mem::size_of::<crate::geometry::LineBox>();
            for line in &layout.lines {
                total += line.text.len();
                total += line.glyphs.len() * std::mem::size_of::<crate::geometry::GlyphBox>();
            }
        }
        total
    }

    /// Returns snapshot of cache metrics.
    pub fn snapshot(&self) -> CacheMetricsSnapshot {
        CacheMetricsSnapshot {
            hits: self.metrics.hit_count(),
            misses: self.metrics.miss_count(),
            evictions: self.metrics.eviction_count(),
            hit_ratio: self.metrics.hit_ratio(),
            entry_count: self.entries.len(),
            approx_memory_bytes: self.approx_memory_bytes(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tok_core::id::FractionalIndex;

    #[test]
    fn test_paragraph_cache_hit_and_miss() {
        let mut cache = ParagraphLayoutCache::new();
        let p = ParagraphNode::new(FractionalIndex::initial(), "body", "שלום עליכם רבותי");
        let style = ParagraphStyle::default();
        let width = 450.0;

        let key = ParagraphLayoutCache::make_key(&p, &style, width);
        assert!(cache.get(&key).is_none());
        assert_eq!(cache.metrics().miss_count(), 1);

        cache.insert(key.clone(), Vec::new());
        assert_eq!(cache.len(), 1);

        let cached = cache.get(&key);
        assert!(cached.is_some());
        assert_eq!(cache.metrics().hit_count(), 1);

        // Edit text -> different key
        let mut edited_p = p.clone();
        edited_p.text.push_str(" עוד מילים");
        let edited_key = ParagraphLayoutCache::make_key(&edited_p, &style, width);
        assert_ne!(key, edited_key);
        assert!(cache.get(&edited_key).is_none());

        // Invalidate node
        assert!(cache.invalidate(p.id));
        assert!(cache.is_empty());
        assert_eq!(cache.metrics().eviction_count(), 1);
    }

    #[test]
    fn test_paragraph_cache_style_and_width_invalidation() {
        let mut cache = ParagraphLayoutCache::new();
        let p = ParagraphNode::new(FractionalIndex::initial(), "body", "טקסט קבוע");
        let style1 = ParagraphStyle::default();
        let mut style2 = ParagraphStyle::default();
        style2.font_size_pt = 14.0;

        let key1 = ParagraphLayoutCache::make_key(&p, &style1, 400.0);
        let key2 = ParagraphLayoutCache::make_key(&p, &style2, 400.0);
        let key3 = ParagraphLayoutCache::make_key(&p, &style1, 500.0);

        cache.insert(key1.clone(), Vec::new());
        assert!(cache.get(&key1).is_some());
        assert!(cache.get(&key2).is_none());
        assert!(cache.get(&key3).is_none());
    }
}
