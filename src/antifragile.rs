//! # Core types and traits for antifragility analysis
//!
//! This module provides the foundational types for analyzing system responses to stress.
//!
//! ## Example: Analyzing a Portfolio
//!
//! ```rust
//! use antifragile::{Antifragile, TriadAnalysis, Triad};
//!
//! struct OptionsPortfolio {
//!     // Long volatility position
//!     vega_exposure: f64,
//! }
//!
//! impl Antifragile for OptionsPortfolio {
//!     type Stressor = f64;  // Market volatility
//!     type Payoff = f64;    // Portfolio P&L
//!
//!     fn payoff(&self, volatility: f64) -> f64 {
//!         // Options gain from volatility (convex payoff)
//!         self.vega_exposure * volatility * volatility
//!     }
//! }
//!
//! let portfolio = OptionsPortfolio { vega_exposure: 1.0 };
//! assert!(portfolio.is_antifragile(0.2, 0.05));
//! ```

use core::cmp::Ordering;
use core::fmt::Display;
use core::ops::{Add, Sub};
use core::str::FromStr;

#[cfg(feature = "std")]
use std::error::Error;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Trait for systems that can be analyzed for fragility
///
/// Implement this trait to measure how your system responds to stress.
pub trait Antifragile {
    /// The type of stressor (e.g., volatility, load, perturbation)
    type Stressor: Copy + Add<Output = Self::Stressor> + Sub<Output = Self::Stressor>;

    /// The type of payoff/outcome (must be comparable and additive)
    type Payoff: Copy + Add<Output = Self::Payoff> + PartialOrd;

    /// The payoff function: what outcome does the system produce under given stress?
    ///
    /// The "payoff" is what you get back when the system experiences
    /// a certain level of stress/volatility.
    fn payoff(&self, stressor: Self::Stressor) -> Self::Payoff;

    /// Returns the payoff added to itself (r + r)
    ///
    /// Used for convexity test: f(x+Δ) + f(x-Δ) vs twin(f(x))
    ///
    /// Named "twin" because it produces the same value twice, added together.
    ///
    /// The default implementation returns `r + r`. Override if your `Payoff` type
    /// has a more efficient doubling operation.
    fn twin(r: Self::Payoff) -> Self::Payoff {
        r + r
    }
}

/// Triad: the three categories of response to volatility
///
/// Variants are ordered by desirability: Fragile < Robust < Antifragile.
/// This ordering is consistent with `Ord`, `rank()`, and numeric conversions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[repr(u8)]
#[must_use]
pub enum Triad {
    /// Harmed by volatility (concave response) - least desirable
    Fragile,
    /// Unaffected by volatility (linear response) - neutral
    Robust,
    /// Benefits from volatility (convex response) - most desirable
    Antifragile,
}

impl Triad {
    /// All variants in desirability order: `[Fragile, Robust, Antifragile]`
    pub const ALL: [Self; 3] = [Self::Fragile, Self::Robust, Self::Antifragile];

    /// Returns an iterator over all variants in desirability order
    #[inline]
    pub fn iter() -> impl Iterator<Item = Self> {
        Self::ALL.into_iter()
    }

    /// Returns the desirability rank: Fragile=0, Robust=1, Antifragile=2
    ///
    /// Higher rank means more desirable. This is consistent with `Ord` ordering.
    #[inline]
    #[must_use]
    pub const fn rank(self) -> u8 {
        self as u8
    }

    /// Returns true if this is the best classification (Antifragile)
    #[inline]
    #[must_use]
    pub const fn is_antifragile(self) -> bool {
        matches!(self, Triad::Antifragile)
    }

    /// Returns true if this is the worst classification (Fragile)
    #[inline]
    #[must_use]
    pub const fn is_fragile(self) -> bool {
        matches!(self, Triad::Fragile)
    }

    /// Returns true if this is neutral (Robust)
    #[inline]
    #[must_use]
    pub const fn is_robust(self) -> bool {
        matches!(self, Triad::Robust)
    }

    /// Returns the opposite classification
    ///
    /// - `Antifragile` ↔ `Fragile`
    /// - `Robust` → `Robust` (self-opposite, as it's neutral)
    #[inline]
    pub const fn opposite(self) -> Self {
        match self {
            Self::Antifragile => Self::Fragile,
            Self::Fragile => Self::Antifragile,
            Self::Robust => Self::Robust,
        }
    }
}

impl PartialOrd for Triad {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Triad {
    /// Orders by desirability: Fragile < Robust < Antifragile
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.rank().cmp(&other.rank())
    }
}

impl Default for Triad {
    /// Returns `Triad::Robust` as the neutral default
    #[inline]
    fn default() -> Self {
        Self::Robust
    }
}

impl From<Triad> for u8 {
    #[inline]
    fn from(triad: Triad) -> Self {
        triad.rank()
    }
}

impl From<Triad> for &'static str {
    #[inline]
    fn from(triad: Triad) -> Self {
        match triad {
            Triad::Antifragile => "antifragile",
            Triad::Fragile => "fragile",
            Triad::Robust => "robust",
        }
    }
}

impl Display for Triad {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Antifragile => write!(f, "Antifragile (benefits from volatility)"),
            Self::Fragile => write!(f, "Fragile (harmed by volatility)"),
            Self::Robust => write!(f, "Robust (unaffected by volatility)"),
        }
    }
}

/// Error returned when converting an invalid value to [`Triad`]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidTriadValue(pub u8);

impl Display for InvalidTriadValue {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "invalid triad value: {} (expected 0, 1, or 2)", self.0)
    }
}

#[cfg(feature = "std")]
impl Error for InvalidTriadValue {}

impl TryFrom<u8> for Triad {
    type Error = InvalidTriadValue;

    #[inline]
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Fragile),
            1 => Ok(Self::Robust),
            2 => Ok(Self::Antifragile),
            n => Err(InvalidTriadValue(n)),
        }
    }
}

/// Error returned when parsing a string into [`Triad`] fails
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseTriadError;

impl Display for ParseTriadError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "invalid triad string (expected \"antifragile\", \"fragile\", or \"robust\")"
        )
    }
}

#[cfg(feature = "std")]
impl Error for ParseTriadError {}

impl FromStr for Triad {
    type Err = ParseTriadError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("antifragile") {
            Ok(Self::Antifragile)
        } else if s.eq_ignore_ascii_case("fragile") {
            Ok(Self::Fragile)
        } else if s.eq_ignore_ascii_case("robust") {
            Ok(Self::Robust)
        } else {
            Err(ParseTriadError)
        }
    }
}

/// Extension trait providing Triad classification methods
pub trait TriadAnalysis: Antifragile {
    /// Classify the system on Taleb's Triad at a specific operating point
    ///
    /// Uses Taleb's convexity test: f(x+Δ) + f(x-Δ) vs 2·f(x)
    /// - If sum > twin → Antifragile (convex payoff)
    /// - If sum < twin → Fragile (concave payoff)
    /// - If sum = twin → Robust (linear payoff)
    ///
    /// # Arguments
    /// * `at` - The operating point (stress level) to test
    /// * `delta` - The perturbation size for the convexity test
    ///
    /// # Note
    /// This uses exact comparison. For floating-point payoffs where exact
    /// equality is unlikely, use [`classify_with_tolerance`](Self::classify_with_tolerance).
    fn classify(&self, at: Self::Stressor, delta: Self::Stressor) -> Triad
    where
        Self::Payoff: Sub<Output = Self::Payoff> + Default + PartialOrd,
    {
        let f_x = self.payoff(at);
        let f_x_plus = self.payoff(at + delta);
        let f_x_minus = self.payoff(at - delta);

        let sum = f_x_plus + f_x_minus;
        let twin_f_x = Self::twin(f_x);

        if sum > twin_f_x {
            Triad::Antifragile
        } else if sum < twin_f_x {
            Triad::Fragile
        } else {
            Triad::Robust
        }
    }

    /// Classify with numerical tolerance for floating-point payoffs
    ///
    /// Like [`classify`](Self::classify), but treats values within `epsilon` of
    /// each other as equal. This is useful for `f32`/`f64` payoffs where exact
    /// equality is rare due to floating-point precision.
    ///
    /// # Arguments
    /// * `at` - The operating point (stress level) to test
    /// * `delta` - The perturbation size for the convexity test
    /// * `epsilon` - Tolerance for considering values equal
    ///
    /// # Example
    ///
    /// ```
    /// use antifragile::{Antifragile, Triad, TriadAnalysis};
    ///
    /// struct NearlyLinear;
    /// impl Antifragile for NearlyLinear {
    ///     type Stressor = f64;
    ///     type Payoff = f64;
    ///     fn payoff(&self, x: Self::Stressor) -> Self::Payoff {
    ///         2.0 * x + 1e-10 * x * x  // Almost linear with tiny convexity
    ///     }
    /// }
    ///
    /// let system = NearlyLinear;
    /// // Exact classification sees the tiny convexity
    /// assert_eq!(system.classify(10.0, 1.0), Triad::Antifragile);
    /// // With tolerance, it's effectively Robust
    /// assert_eq!(system.classify_with_tolerance(10.0, 1.0, 1e-6), Triad::Robust);
    /// ```
    fn classify_with_tolerance(
        &self,
        at: Self::Stressor,
        delta: Self::Stressor,
        epsilon: Self::Payoff,
    ) -> Triad
    where
        Self::Payoff: Sub<Output = Self::Payoff> + Default + PartialOrd,
    {
        let f_x = self.payoff(at);
        let f_x_plus = self.payoff(at + delta);
        let f_x_minus = self.payoff(at - delta);

        let sum = f_x_plus + f_x_minus;
        let twin_f_x = Self::twin(f_x);

        // Compute absolute difference: |sum - twin_f_x|
        let diff = if sum >= twin_f_x {
            sum - twin_f_x
        } else {
            twin_f_x - sum
        };

        if diff <= epsilon {
            Triad::Robust
        } else if sum > twin_f_x {
            Triad::Antifragile
        } else {
            Triad::Fragile
        }
    }

    /// Check if system is antifragile at a given point (convexity test)
    #[must_use]
    fn is_antifragile(&self, at: Self::Stressor, delta: Self::Stressor) -> bool
    where
        Self::Payoff: Sub<Output = Self::Payoff> + Default + PartialOrd,
    {
        self.classify(at, delta) == Triad::Antifragile
    }

    /// Does the system gain from increased stress?
    ///
    /// A practical test: does higher stress lead to better payoff?
    /// Returns true if payoff(high) > payoff(low).
    ///
    /// This is useful for learning systems where payoff improves
    /// with exposure, even if mathematically concave.
    #[must_use]
    fn gains_from_stress(&self, low: Self::Stressor, high: Self::Stressor) -> bool {
        self.payoff(high) > self.payoff(low)
    }

    /// Is the payoff stable across stress levels?
    ///
    /// Returns true if the absolute difference between `payoff(high)` and
    /// `payoff(low)` is less than or equal to `threshold`.
    ///
    /// This indicates robust behavior where the system's output doesn't
    /// vary significantly with changes in stress.
    ///
    /// # Example
    ///
    /// A system with constant payoff is perfectly stable:
    /// ```
    /// use antifragile::{Antifragile, TriadAnalysis};
    ///
    /// struct ConstantSystem;
    /// impl Antifragile for ConstantSystem {
    ///     type Stressor = f64;
    ///     type Payoff = f64;
    ///     fn payoff(&self, _: Self::Stressor) -> Self::Payoff { 10.0 }
    /// }
    ///
    /// let system = ConstantSystem;
    /// assert!(system.is_stable(1.0, 100.0, 0.001));
    /// ```
    #[must_use]
    fn is_stable(&self, low: Self::Stressor, high: Self::Stressor, threshold: Self::Payoff) -> bool
    where
        Self::Payoff: Sub<Output = Self::Payoff>,
    {
        let payoff_low = self.payoff(low);
        let payoff_high = self.payoff(high);

        // Check |payoff_high - payoff_low| <= threshold
        if payoff_high >= payoff_low {
            payoff_high - payoff_low <= threshold
        } else {
            payoff_low - payoff_high <= threshold
        }
    }
}

// Blanket implementation for all Antifragile types
impl<T: Antifragile> TriadAnalysis for T {}

/// A wrapper that marks a system as verified on the Triad
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Verified<T> {
    inner: T,
    classification: Triad,
}

impl<T: Antifragile> Verified<T>
where
    T::Payoff: Sub<Output = T::Payoff> + Default + PartialOrd,
{
    /// Verify a system's Triad classification at a given operating point
    #[must_use]
    pub fn check(system: T, at: T::Stressor, delta: T::Stressor) -> Self {
        let classification = system.classify(at, delta);
        Self {
            inner: system,
            classification,
        }
    }

    /// Get the verified Triad classification
    #[inline]
    pub const fn classification(&self) -> Triad {
        self.classification
    }

    /// Get reference to inner system
    #[inline]
    #[must_use]
    pub const fn inner(&self) -> &T {
        &self.inner
    }

    /// Unwrap the verified system
    #[inline]
    #[must_use]
    pub fn into_inner(self) -> T {
        self.inner
    }

    /// Returns true if the system was classified as Antifragile
    #[inline]
    #[must_use]
    pub const fn is_antifragile(&self) -> bool {
        self.classification.is_antifragile()
    }

    /// Returns true if the system was classified as Fragile
    #[inline]
    #[must_use]
    pub const fn is_fragile(&self) -> bool {
        self.classification.is_fragile()
    }

    /// Returns true if the system was classified as Robust
    #[inline]
    #[must_use]
    pub const fn is_robust(&self) -> bool {
        self.classification.is_robust()
    }

    /// Re-verify classification at a new operating point
    ///
    /// Updates the stored classification by re-running the convexity test
    /// at the specified operating point and delta.
    #[inline]
    pub fn re_verify(&mut self, at: T::Stressor, delta: T::Stressor) {
        self.classification = self.inner.classify(at, delta);
    }

    /// Check if the classification still holds at a different operating point
    ///
    /// Returns `true` if classifying at the new point yields the same result
    /// as the stored classification.
    #[inline]
    #[must_use]
    pub fn still_holds(&self, at: T::Stressor, delta: T::Stressor) -> bool {
        self.inner.classify(at, delta) == self.classification
    }
}

impl<T> AsRef<T> for Verified<T> {
    #[inline]
    fn as_ref(&self) -> &T {
        &self.inner
    }
}

impl<T> core::ops::Deref for Verified<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: Antifragile + Default> Default for Verified<T>
where
    T::Stressor: Default,
    T::Payoff: Sub<Output = T::Payoff> + Default + PartialOrd,
{
    /// Creates a verified system using `T::default()` classified at the default stressor
    fn default() -> Self {
        let system = T::default();
        let at = T::Stressor::default();
        Self::check(system, at, at)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test helpers - mathematical functions for verifying the convexity test
    struct ConvexFn; // f(x) = x²
    struct ConcaveFn; // f(x) = √x
    struct LinearFn {
        slope: f64,
        intercept: f64,
    }

    impl Antifragile for ConvexFn {
        type Stressor = f64;
        type Payoff = f64;
        fn payoff(&self, x: Self::Stressor) -> Self::Payoff {
            x * x
        }
    }

    impl Antifragile for ConcaveFn {
        type Stressor = f64;
        type Payoff = f64;
        fn payoff(&self, x: Self::Stressor) -> Self::Payoff {
            x.abs().sqrt()
        }
    }

    impl Antifragile for LinearFn {
        type Stressor = f64;
        type Payoff = f64;
        fn payoff(&self, x: Self::Stressor) -> Self::Payoff {
            self.slope * x + self.intercept
        }
    }

    #[test]
    fn test_convex_is_antifragile() {
        let system = ConvexFn;
        assert!(system.is_antifragile(10.0, 1.0));
        assert_eq!(system.classify(10.0, 1.0), Triad::Antifragile);
    }

    #[test]
    fn test_concave_is_fragile() {
        let system = ConcaveFn;
        assert_eq!(system.classify(10.0, 1.0), Triad::Fragile);
    }

    #[test]
    fn test_linear_is_robust() {
        let system = LinearFn {
            slope: 2.0,
            intercept: 5.0,
        };
        assert_eq!(system.classify(10.0, 1.0), Triad::Robust);
    }

    #[test]
    fn test_gains_from_stress() {
        let convex = ConvexFn;
        assert!(convex.gains_from_stress(1.0, 2.0)); // 1 < 4

        let concave = ConcaveFn;
        assert!(concave.gains_from_stress(1.0, 4.0)); // 1 < 2
    }

    #[test]
    fn test_verified_wrapper() {
        let system = ConvexFn;
        let verified = Verified::check(system, 10.0, 1.0);
        assert_eq!(verified.classification(), Triad::Antifragile);
    }

    #[test]
    fn test_triad_display() {
        assert_eq!(
            format!("{}", Triad::Antifragile),
            "Antifragile (benefits from volatility)"
        );
        assert_eq!(
            format!("{}", Triad::Fragile),
            "Fragile (harmed by volatility)"
        );
        assert_eq!(
            format!("{}", Triad::Robust),
            "Robust (unaffected by volatility)"
        );
    }

    #[test]
    fn test_triad_ordering() {
        // Ordering by desirability: Fragile < Robust < Antifragile
        assert!(Triad::Fragile < Triad::Robust);
        assert!(Triad::Robust < Triad::Antifragile);
        assert!(Triad::Fragile < Triad::Antifragile);

        // Test rank values (matches desirability order)
        assert_eq!(Triad::Fragile.rank(), 0);
        assert_eq!(Triad::Robust.rank(), 1);
        assert_eq!(Triad::Antifragile.rank(), 2);

        // Test sorting (sorts by desirability, worst to best)
        let mut triads = vec![Triad::Antifragile, Triad::Fragile, Triad::Robust];
        triads.sort();
        assert_eq!(
            triads,
            vec![Triad::Fragile, Triad::Robust, Triad::Antifragile]
        );
    }

    #[test]
    fn test_triad_predicates() {
        assert!(Triad::Antifragile.is_antifragile());
        assert!(!Triad::Antifragile.is_fragile());
        assert!(!Triad::Antifragile.is_robust());

        assert!(Triad::Fragile.is_fragile());
        assert!(!Triad::Fragile.is_antifragile());
        assert!(!Triad::Fragile.is_robust());

        assert!(Triad::Robust.is_robust());
        assert!(!Triad::Robust.is_antifragile());
        assert!(!Triad::Robust.is_fragile());
    }

    #[test]
    fn test_verified_predicates() {
        let system = ConvexFn;
        let verified = Verified::check(system, 10.0, 1.0);
        assert!(verified.is_antifragile());
        assert!(!verified.is_fragile());
        assert!(!verified.is_robust());
    }

    #[test]
    fn test_triad_default() {
        assert_eq!(Triad::default(), Triad::Robust);
    }

    #[test]
    fn test_triad_from_u8() {
        assert_eq!(Triad::try_from(0_u8), Ok(Triad::Fragile));
        assert_eq!(Triad::try_from(1_u8), Ok(Triad::Robust));
        assert_eq!(Triad::try_from(2_u8), Ok(Triad::Antifragile));
        assert_eq!(Triad::try_from(3_u8), Err(InvalidTriadValue(3)));
        assert_eq!(Triad::try_from(255_u8), Err(InvalidTriadValue(255)));
    }

    #[test]
    fn test_triad_into_u8() {
        assert_eq!(u8::from(Triad::Fragile), 0);
        assert_eq!(u8::from(Triad::Robust), 1);
        assert_eq!(u8::from(Triad::Antifragile), 2);
    }

    #[test]
    fn test_triad_into_str() {
        assert_eq!(<&str>::from(Triad::Antifragile), "antifragile");
        assert_eq!(<&str>::from(Triad::Fragile), "fragile");
        assert_eq!(<&str>::from(Triad::Robust), "robust");
    }

    #[test]
    fn test_verified_deref() {
        let system = ConvexFn;
        let verified = Verified::check(system, 10.0, 1.0);
        // Can call payoff through Deref
        assert!((verified.payoff(5.0) - 25.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_invalid_triad_value_display() {
        let err = InvalidTriadValue(42);
        assert_eq!(
            format!("{err}"),
            "invalid triad value: 42 (expected 0, 1, or 2)"
        );
    }

    #[test]
    fn test_triad_from_str() {
        // Case insensitive parsing
        assert_eq!("antifragile".parse::<Triad>(), Ok(Triad::Antifragile));
        assert_eq!("Antifragile".parse::<Triad>(), Ok(Triad::Antifragile));
        assert_eq!("ANTIFRAGILE".parse::<Triad>(), Ok(Triad::Antifragile));

        assert_eq!("fragile".parse::<Triad>(), Ok(Triad::Fragile));
        assert_eq!("Fragile".parse::<Triad>(), Ok(Triad::Fragile));

        assert_eq!("robust".parse::<Triad>(), Ok(Triad::Robust));
        assert_eq!("ROBUST".parse::<Triad>(), Ok(Triad::Robust));

        // Invalid strings
        assert_eq!("invalid".parse::<Triad>(), Err(ParseTriadError));
        assert_eq!("".parse::<Triad>(), Err(ParseTriadError));
    }

    #[test]
    fn test_parse_triad_error_display() {
        let err = ParseTriadError;
        assert_eq!(
            format!("{err}"),
            "invalid triad string (expected \"antifragile\", \"fragile\", or \"robust\")"
        );
    }

    #[test]
    fn test_classify_at_zero() {
        let system = ConvexFn;
        let _ = system.classify(0.0, 0.1);
    }

    #[test]
    fn test_classify_with_zero_delta() {
        let system = ConvexFn;
        assert_eq!(system.classify(10.0, 0.0), Triad::Robust);
    }

    #[test]
    fn test_classify_negative_stressor() {
        let system = ConvexFn;
        assert_eq!(system.classify(-10.0, 1.0), Triad::Antifragile);
    }

    #[test]
    fn test_triad_opposite() {
        assert_eq!(Triad::Antifragile.opposite(), Triad::Fragile);
        assert_eq!(Triad::Fragile.opposite(), Triad::Antifragile);
        assert_eq!(Triad::Robust.opposite(), Triad::Robust);
        assert_eq!(Triad::Antifragile.opposite().opposite(), Triad::Antifragile);
    }

    #[test]
    fn test_triad_iter() {
        let all: Vec<_> = Triad::iter().collect();
        assert_eq!(all, vec![Triad::Fragile, Triad::Robust, Triad::Antifragile]);
        assert_eq!(Triad::ALL.len(), 3);
    }
}
