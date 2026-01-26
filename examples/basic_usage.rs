//! Basic usage example for the antifragile crate.
//!
//! Run with: `cargo run --example basic_usage`

use antifragile::{Antifragile, Triad, TriadAnalysis, Verified};

/// A convex system: f(x) = x²
/// Benefits from volatility (antifragile)
struct ConvexSystem;

impl Antifragile for ConvexSystem {
    type Stressor = f64;
    type Payoff = f64;

    fn payoff(&self, x: Self::Stressor) -> Self::Payoff {
        x * x
    }
}

/// A concave system: f(x) = √x
/// Harmed by volatility (fragile)
struct ConcaveSystem;

impl Antifragile for ConcaveSystem {
    type Stressor = f64;
    type Payoff = f64;

    fn payoff(&self, x: Self::Stressor) -> Self::Payoff {
        x.abs().sqrt()
    }
}

/// A linear system: f(x) = 2x + 5
/// Unaffected by volatility (robust)
struct LinearSystem;

impl Antifragile for LinearSystem {
    type Stressor = f64;
    type Payoff = f64;

    fn payoff(&self, x: Self::Stressor) -> Self::Payoff {
        2.0 * x + 5.0
    }
}

fn main() {
    println!("=== Antifragile Library Demo ===\n");

    // Create systems
    let convex = ConvexSystem;
    let concave = ConcaveSystem;
    let linear = LinearSystem;

    // Classify each system
    let at = 10.0; // Operating point
    let delta = 1.0; // Perturbation size

    println!("Testing at operating point {at} with delta {delta}:\n");

    // Convex system (antifragile)
    let classification = convex.classify(at, delta);
    println!("Convex system (x²):");
    println!("  f({}) = {}", at - delta, convex.payoff(at - delta));
    println!("  f({}) = {}", at, convex.payoff(at));
    println!("  f({}) = {}", at + delta, convex.payoff(at + delta));
    println!("  Classification: {classification}");
    println!();

    // Concave system (fragile)
    let classification = concave.classify(at, delta);
    println!("Concave system (√x):");
    println!("  f({}) = {:.4}", at - delta, concave.payoff(at - delta));
    println!("  f({}) = {:.4}", at, concave.payoff(at));
    println!("  f({}) = {:.4}", at + delta, concave.payoff(at + delta));
    println!("  Classification: {classification}");
    println!();

    // Linear system (robust)
    let classification = linear.classify(at, delta);
    println!("Linear system (2x + 5):");
    println!("  f({}) = {}", at - delta, linear.payoff(at - delta));
    println!("  f({}) = {}", at, linear.payoff(at));
    println!("  f({}) = {}", at + delta, linear.payoff(at + delta));
    println!("  Classification: {classification}");
    println!();

    // Demonstrate Verified wrapper
    println!("=== Using Verified Wrapper ===\n");
    let verified = Verified::check(ConvexSystem, at, delta);
    println!("Verified convex system:");
    println!("  Classification: {}", verified.classification());
    println!("  Is antifragile: {}", verified.is_antifragile());
    println!("  Payoff at 5.0: {}", verified.payoff(5.0));
    println!();

    // Demonstrate ordering
    println!("=== Triad Ordering ===\n");
    let mut triads = vec![Triad::Antifragile, Triad::Fragile, Triad::Robust];
    println!("Before sorting: {triads:?}");
    triads.sort();
    println!("After sorting:  {triads:?}");
    println!("(Fragile < Robust < Antifragile)");
    println!();

    // Demonstrate conversions
    println!("=== Conversions ===\n");
    let triad = Triad::Antifragile;
    let s: &str = triad.into();
    let n: u8 = triad.into();
    println!("Triad::Antifragile -> &str: \"{s}\"");
    println!("Triad::Antifragile -> u8: {n}");
    println!();

    let parsed: Triad = "robust".parse().unwrap();
    println!("\"robust\".parse() -> {parsed:?}");

    let from_u8 = Triad::try_from(1u8).unwrap();
    println!("Triad::try_from(1u8) -> {from_u8:?}");
}
