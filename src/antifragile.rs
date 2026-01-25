use std::cmp::Ordering;
use std::error::Error;
use std::fmt::Display;
use std::ops::{Add, Sub};

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
    fn twin(r: Self::Payoff) -> Self::Payoff;
}

/// Triad: the three categories of response to volatility
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[must_use]
pub enum Triad {
    /// Benefits from volatility (convex response)
    Antifragile,
    /// Harmed by volatility (concave response)
    Fragile,
    /// Unaffected by volatility (linear response)
    Robust,
}

impl Triad {
    /// Returns the numeric rank: Fragile=0, Robust=1, Antifragile=2
    #[inline]
    #[must_use]
    pub const fn rank(self) -> u8 {
        match self {
            Triad::Fragile => 0,
            Triad::Robust => 1,
            Triad::Antifragile => 2,
        }
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
}

impl PartialOrd for Triad {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Triad {
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

impl Display for Triad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fragile => write!(f, "Fragile (harmed by volatility)"),
            Self::Robust => write!(f, "Robust (unaffected by volatility)"),
            Self::Antifragile => write!(f, "Antifragile (benefits from volatility)"),
        }
    }
}

/// Error returned when converting an invalid value to [`Triad`]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidTriadValue(pub u8);

impl Display for InvalidTriadValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid triad value: {} (expected 0, 1, or 2)", self.0)
    }
}

impl Error for InvalidTriadValue {}

impl TryFrom<u8> for Triad {
    type Error = InvalidTriadValue;

    #[inline]
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Antifragile),
            1 => Ok(Self::Fragile),
            2 => Ok(Self::Robust),
            n => Err(InvalidTriadValue(n)),
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
    /// This is the formal mathematical definition. For learning
    /// systems, also check `gains_from_stress()`.
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
    /// Returns true if payoff varies by less than threshold.
    /// Indicates robust behavior.
    #[must_use]
    fn is_stable(&self, low: Self::Stressor, high: Self::Stressor, threshold: Self::Payoff) -> bool
    where
        Self::Payoff: Sub<Output = Self::Payoff>,
    {
        let diff = self.payoff(high) + Self::twin(self.payoff(low));
        let sum = self.payoff(low) + Self::twin(self.payoff(high));
        diff < sum + threshold && sum < diff + threshold
    }
}

// Blanket implementation for all Antifragile types
impl<T: Antifragile> TriadAnalysis for T {}

/// A wrapper that marks a system as verified on the Triad
#[derive(Debug, Clone)]
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

    /// Returns true if the system was classified as Fragile
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
}

impl<T> AsRef<T> for Verified<T> {
    #[inline]
    fn as_ref(&self) -> &T {
        &self.inner
    }
}

impl<T> std::ops::Deref for Verified<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
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
        fn twin(r: Self::Payoff) -> Self::Payoff {
            r + r
        }
    }

    impl Antifragile for ConcaveFn {
        type Stressor = f64;
        type Payoff = f64;
        fn payoff(&self, x: Self::Stressor) -> Self::Payoff {
            x.abs().sqrt()
        }
        fn twin(r: Self::Payoff) -> Self::Payoff {
            r + r
        }
    }

    impl Antifragile for LinearFn {
        type Stressor = f64;
        type Payoff = f64;
        fn payoff(&self, x: Self::Stressor) -> Self::Payoff {
            self.slope * x + self.intercept
        }
        fn twin(r: Self::Payoff) -> Self::Payoff {
            r + r
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
        // Fragile < Robust < Antifragile
        assert!(Triad::Fragile < Triad::Robust);
        assert!(Triad::Robust < Triad::Antifragile);
        assert!(Triad::Fragile < Triad::Antifragile);

        // Test rank values
        assert_eq!(Triad::Fragile.rank(), 0);
        assert_eq!(Triad::Robust.rank(), 1);
        assert_eq!(Triad::Antifragile.rank(), 2);

        // Test sorting
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
    fn test_verified_deref() {
        let system = ConvexFn;
        let verified = Verified::check(system, 10.0, 1.0);
        // Can call payoff through Deref
        assert_eq!(verified.payoff(5.0), 25.0);
    }

    #[test]
    fn test_invalid_triad_value_display() {
        let err = InvalidTriadValue(42);
        assert_eq!(
            format!("{err}"),
            "invalid triad value: 42 (expected 0, 1, or 2)"
        );
    }
}
