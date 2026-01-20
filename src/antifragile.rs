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
