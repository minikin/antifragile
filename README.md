# Antifragile

[![Crates.io](https://img.shields.io/crates/v/antifragile.svg)](https://crates.io/crates/antifragile)
[![Documentation](https://docs.rs/antifragile/badge.svg)](https://docs.rs/antifragile)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![CI](https://github.com/minikin/antifragile/actions/workflows/ci.yml/badge.svg)](https://github.com/minikin/antifragile/actions/workflows/ci.yml)

A Rust library implementing [Nassim Nicholas Taleb's](https://en.wikipedia.org/wiki/Nassim_Nicholas_Taleb) [antifragility theory](https://en.wikipedia.org/wiki/Antifragile).

## Overview

This library provides a trait-based system for analyzing how systems respond to stress and
volatility, classifying them into three categories:

- **Antifragile**: Benefits from volatility ([convex response](https://en.wikipedia.org/wiki/Convex_function))
- **Fragile**: Harmed by volatility ([concave response](https://en.wikipedia.org/wiki/Concave_function))
- **Robust**: Unaffected by volatility ([linear response](https://en.wikipedia.org/wiki/Linear_function))

## Installation

```toml
[dependencies]
antifragile = "0.0.1"
```

## Quick Start

```rust
use antifragile::{Antifragile, Triad, TriadAnalysis};

// Define a system with convex response (benefits from volatility)
struct ConvexSystem;

impl Antifragile for ConvexSystem {
    type Stressor = f64;
    type Payoff = f64;

    fn payoff(&self, x: Self::Stressor) -> Self::Payoff {
        x * x  // Quadratic: convex response
    }
}

let system = ConvexSystem;
assert_eq!(system.classify(10.0, 1.0), Triad::Antifragile);
```

## Core Concepts

### The Antifragile Trait

Implement the `Antifragile` trait to define how your system responds to stress:

```rust
use antifragile::Antifragile;

struct MySystem;

impl Antifragile for MySystem {
    type Stressor = f64;  // The type of stress applied
    type Payoff = f64;    // The outcome produced

    fn payoff(&self, stressor: Self::Stressor) -> Self::Payoff {
        // Define your system's response to stress
        stressor * stressor
    }
}
```

### The Triad Classification

The `Triad` enum represents the three categories:

```rust
use antifragile::Triad;

let classification = Triad::Antifragile;

// Check classification
assert!(classification.is_antifragile());

// Ordering: Fragile < Robust < Antifragile
assert!(Triad::Fragile < Triad::Robust);
assert!(Triad::Robust < Triad::Antifragile);

// Convert to/from strings
let s: &str = classification.into();  // "antifragile"
let parsed: Triad = "robust".parse().unwrap();

// Convert to/from u8 (Fragile=0, Robust=1, Antifragile=2)
let n: u8 = classification.into();  // 2
let from_n = Triad::try_from(1u8).unwrap();  // Triad::Robust
```

### The Verified Wrapper

Use `Verified` to wrap a system with its verified classification:

```rust
use antifragile::{Antifragile, Verified, TriadAnalysis};

struct MySystem;
impl Antifragile for MySystem {
    type Stressor = f64;
    type Payoff = f64;
    fn payoff(&self, x: Self::Stressor) -> Self::Payoff { x * x }
}

let verified = Verified::check(MySystem, 10.0, 1.0);
println!("Classification: {}", verified.classification());
```

## Mathematical Foundation

The classification is based on **second-order effects** (convexity):

For a payoff function f(x) at operating point x with perturbation δ:

- **Convex (Antifragile)**: f(x+δ) + f(x-δ) > 2·f(x)
- **Concave (Fragile)**: f(x+δ) + f(x-δ) < 2·f(x)
- **Linear (Robust)**: f(x+δ) + f(x-δ) = 2·f(x)

This is [Jensen's inequality](https://en.wikipedia.org/wiki/Jensen%27s_inequality) applied to volatility.

## When to Use This Library

**Good fit:**
- Analyzing financial instruments (options, insurance)
- Evaluating system resilience in chaos engineering
- Comparing algorithms under varying load
- Educational purposes (demonstrating Taleb's theory)

**Not a good fit:**
- Real-time trading decisions (too abstract)
- Systems where "stress" is not mathematically quantifiable
- Cases requiring probabilistic analysis (use Monte Carlo instead)

## Feature Flags

| Feature | Default | Description                                                         |
| ------- | ------- | ------------------------------------------------------------------- |
| `std`   | Yes     | Enable standard library support. Disable for `no_std` environments. |
| `serde` | No      | Enable serialization/deserialization for `Triad` and `Verified`.    |

### Using in `no_std` environments

```toml
[dependencies]
antifragile = { version = "0.0.1", default-features = false }
```

### Enabling serde support

```toml
[dependencies]
antifragile = { version = "0.0.1", features = ["serde"] }
```

## Minimum Supported Rust Version

This crate requires Rust 1.85 or later (edition 2024).

## License

Licensed under the MIT License. See [LICENSE](LICENSE) for details.
