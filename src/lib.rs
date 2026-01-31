//! # Antifragile
//!
//! A Rust library for analyzing system responses to stress based on
//! Nassim Nicholas Taleb's antifragility theory.
//!
//! ## Theory Background
//!
//! Antifragility goes beyond resilience or robustness. While resilient systems
//! resist shocks and stay the same, antifragile systems actually benefit from
//! volatility, randomness, and stressors.
//!
//! The mathematical foundation is **convexity**:
//! - **Convex functions** (like x²) benefit from variance (antifragile)
//! - **Concave functions** (like √x) are harmed by variance (fragile)
//! - **Linear functions** (like 2x) are unaffected by variance (robust)
//!
//! ## Core Components
//!
//! | Component | Purpose |
//! |-----------|---------|
//! | [`Antifragile`] | Trait for systems with payoff functions |
//! | [`Triad`] | Classification enum (Fragile/Robust/Antifragile) |
//! | [`TriadAnalysis`] | Extension trait with classification methods |
//! | [`Verified`] | Wrapper that caches classification result |
//!
//! ## Performance Characteristics
//!
//! All operations are O(1) with no heap allocations in the core path:
//! - `classify()`: 3 payoff evaluations + comparisons
//! - `Triad` operations: all constant-time
//! - `Verified::check()`: one classification + struct creation
//!
//! ## Quick Start
//!
//! ```rust
//! use antifragile::{Antifragile, Triad, TriadAnalysis};
//!
//! // Define a system with convex response (benefits from volatility)
//! struct ConvexSystem;
//!
//! impl Antifragile for ConvexSystem {
//!     type Stressor = f64;
//!     type Payoff = f64;
//!
//!     fn payoff(&self, x: Self::Stressor) -> Self::Payoff {
//!         x * x  // Quadratic: convex response
//!     }
//! }
//!
//! let system = ConvexSystem;
//! assert_eq!(system.classify(10.0, 1.0), Triad::Antifragile);
//! ```
//!
//! ## Mathematical Foundation
//!
//! The classification is based on **second-order effects** (convexity):
//!
//! For a payoff function f(x) at operating point x with perturbation δ:
//!
//! - **Convex (Antifragile)**: f(x+δ) + f(x-δ) > 2·f(x)
//! - **Concave (Fragile)**: f(x+δ) + f(x-δ) < 2·f(x)
//! - **Linear (Robust)**: f(x+δ) + f(x-δ) = 2·f(x)
//!
//! This is Jensen's inequality applied to volatility.
//!
//! ## Examples
//!
//! ### Financial Options (Antifragile)
//!
//! Options have convex payoffs - they benefit from volatility:
//!
//! ```rust
//! use antifragile::{Antifragile, TriadAnalysis, Triad};
//!
//! struct CallOption {
//!     strike: f64,
//! }
//!
//! impl Antifragile for CallOption {
//!     type Stressor = f64;  // Underlying price
//!     type Payoff = f64;    // Option value
//!
//!     fn payoff(&self, price: f64) -> f64 {
//!         (price - self.strike).max(0.0)
//!     }
//! }
//!
//! let option = CallOption { strike: 100.0 };
//! assert_eq!(option.classify(100.0, 10.0), Triad::Antifragile);
//! ```
//!
//! ### Insurance Portfolio (Fragile)
//!
//! Traditional insurance has concave payoffs - harmed by volatility:
//!
//! ```rust
//! use antifragile::{Antifragile, TriadAnalysis, Triad};
//!
//! struct InsurancePortfolio {
//!     premium_collected: f64,
//!     max_liability: f64,
//! }
//!
//! impl Antifragile for InsurancePortfolio {
//!     type Stressor = f64;  // Claim rate
//!     type Payoff = f64;    // Profit
//!
//!     fn payoff(&self, claim_rate: f64) -> f64 {
//!         self.premium_collected - (claim_rate * self.max_liability).min(self.max_liability)
//!     }
//! }
//!
//! let portfolio = InsurancePortfolio {
//!     premium_collected: 1000.0,
//!     max_liability: 10000.0,
//! };
//! assert_eq!(portfolio.classify(0.05, 0.02), Triad::Fragile);
//! ```
//!
//! ## When to Use This Library
//!
//! **Good fit:**
//! - Analyzing financial instruments (options, insurance)
//! - Evaluating system resilience in chaos engineering
//! - Comparing algorithms under varying load
//! - Educational purposes (demonstrating Taleb's theory)
//!
//! **Not a good fit:**
//! - Real-time trading decisions (too abstract)
//! - Systems where "stress" is not mathematically quantifiable
//! - Cases requiring probabilistic analysis (use Monte Carlo instead)
//!
//! ## Feature Flags
//!
//! | Feature | Default | Description |
//! |---------|---------|-------------|
//! | `std` | Yes | Standard library support (disable for `no_std`) |
//! | `serde` | No | Serialization support for `Triad` and `Verified` |
#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

/// Core types and traits for antifragility analysis.
pub mod antifragile;

pub use antifragile::{
    Antifragile, InvalidTriadValue, ParseTriadError, Triad, TriadAnalysis, Verified,
};

/// Common f64-based Antifragile systems
pub mod prelude {
    pub use super::{Antifragile, Triad, TriadAnalysis, Verified};

    /// Type alias for f64 stressor/payoff classification results
    pub type ClassifyResult = super::Triad;
}
