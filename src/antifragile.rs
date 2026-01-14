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
