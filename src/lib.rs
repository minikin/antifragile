//! #  Antifragile - A Rust library for analyzing system responses to stress.
//!
//! This library implements concepts from Nassim Nicholas Taleb's antifragility theory.
//!
//! ## Overview
//!
//! This library provides a trait-based system for analyzing how systems respond
//! to stress and volatility, classifying them into three categories:
//!
//! - **Antifragile**: Benefits from volatility (convex response)
//! - **Fragile**: Harmed by volatility (concave response)
//! - **Robust**: Unaffected by volatility (linear response)
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
//!     // twin() uses default implementation: r + r
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
//! - `serde`: Enable serialization/deserialization for `Triad` and `Verified`
//! - `std`: Enable standard library support (enabled by default). Disable for `no_std` environments.
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
