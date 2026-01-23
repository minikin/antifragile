use std::cmp::Ordering;
use std::error::Error;
use std::fmt::Display;
use std::ops::{Add, Sub};

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
}
