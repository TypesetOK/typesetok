//! Recto / Verso spread orientation helpers for RTL Hebrew books.
//!
//! In Hebrew RTL books:
//! - Odd pages (1, 3, 5, ...) are **Recto** (left side of the open book spread):
//!   the binding spine is on the **right** edge (`x -> 1.0`) of the page!
//!   Wait: in `tok_typeset::multi_flow::SpreadSide`:
//!   let's stay consistent with `tok-typeset` while also supporting standard RTL spine parity.

use serde::{Deserialize, Serialize};
use tok_typeset::multi_flow::SpreadSide as TypesetSpreadSide;

/// Spread side of a physical page in an RTL book.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpreadSide {
    /// Odd-numbered page (1, 3, 5...).
    #[default]
    Recto,
    /// Even-numbered page (2, 4, 6...).
    Verso,
}

impl SpreadSide {
    /// Infers spread side from a 1-based page index.
    #[inline]
    pub const fn from_page_index(page_index: usize) -> Self {
        if page_index % 2 == 1 {
            Self::Recto
        } else {
            Self::Verso
        }
    }

    /// Converts to `tok_typeset::multi_flow::SpreadSide`.
    #[inline]
    pub const fn to_typeset_spread_side(self) -> TypesetSpreadSide {
        match self {
            Self::Recto => TypesetSpreadSide::Recto,
            Self::Verso => TypesetSpreadSide::Verso,
        }
    }

    /// Returns whether the inner spine edge is on the right (`x > 0.5`) or left (`x < 0.5`)
    /// in RTL Hebrew book binding (Recto = left-hand page of spread -> spine on right).
    #[inline]
    pub const fn is_spine_on_right_rtl(self) -> bool {
        matches!(self, Self::Recto)
    }

    /// Returns the normalized X coordinate of the spine edge (`1.0` for right, `0.0` for left).
    #[inline]
    pub const fn spine_x_rtl(self) -> f32 {
        if self.is_spine_on_right_rtl() {
            1.0
        } else {
            0.0
        }
    }
}
