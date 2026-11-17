//! Deterministic integer millipoint arithmetic (`1 pt = 1000 mpt`).
//!
//! Used by the continuous column engine and 5-stage balance solver to avoid
//! cumulative IEEE-754 rounding drift when comparing column heights against
//! strict typographic thresholds (e.g. `2.0 pt = 2000 mpt`).

use serde::{Deserialize, Serialize};
use std::iter::Sum;
use std::ops::{Add, AddAssign, Div, Mul, Sub, SubAssign};

pub const MPT_PER_PT: i64 = 1_000;

/// Fixed-point length in millipoints (`1 pt = 1000 mpt`).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
pub struct Mpt(pub i64);

impl Mpt {
    pub const ZERO: Self = Self(0);

    #[inline]
    pub fn from_pt(pt: f32) -> Self {
        if !pt.is_finite() {
            return Self::ZERO;
        }
        Self((pt as f64 * MPT_PER_PT as f64).round() as i64)
    }

    #[inline]
    pub fn from_pt_f64(pt: f64) -> Self {
        if !pt.is_finite() {
            return Self::ZERO;
        }
        Self((pt * MPT_PER_PT as f64).round() as i64)
    }

    #[inline]
    pub fn to_pt(self) -> f32 {
        (self.0 as f64 / MPT_PER_PT as f64) as f32
    }

    #[inline]
    pub fn to_pt_f64(self) -> f64 {
        self.0 as f64 / MPT_PER_PT as f64
    }

    #[inline]
    pub fn abs_diff(self, other: Self) -> Self {
        Self((self.0 - other.0).abs())
    }

    #[inline]
    pub fn saturating_sub(self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }
}

impl Add for Mpt {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0.saturating_add(rhs.0))
    }
}

impl Sub for Mpt {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0.saturating_sub(rhs.0))
    }
}

impl AddAssign for Mpt {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.0 = self.0.saturating_add(rhs.0);
    }
}

impl SubAssign for Mpt {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        self.0 = self.0.saturating_sub(rhs.0);
    }
}

impl Mul<i64> for Mpt {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: i64) -> Self::Output {
        Self(self.0.saturating_mul(rhs))
    }
}

impl Div<i64> for Mpt {
    type Output = Self;
    #[inline]
    fn div(self, rhs: i64) -> Self::Output {
        if rhs == 0 {
            Self::ZERO
        } else {
            Self(self.0 / rhs)
        }
    }
}

impl Sum for Mpt {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::ZERO, |acc, v| acc + v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mpt_roundtrip_and_arithmetic() {
        let a = Mpt::from_pt(14.5);
        let b = Mpt::from_pt(2.0);
        assert_eq!(a.0, 14_500);
        assert_eq!(b.0, 2_000);
        assert_eq!((a - b).to_pt(), 12.5);
        assert_eq!(a.abs_diff(Mpt::from_pt(16.5)), Mpt(2_000));
    }
}
