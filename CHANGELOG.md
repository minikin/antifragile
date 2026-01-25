# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.1] - 2025-01-25

### Added

- `Antifragile` trait for defining system responses to stress
  - `Stressor` associated type for the type of stress applied
  - `Payoff` associated type for the outcome produced
  - `payoff()` method to compute system response to stressor
  - `twin()` method with default implementation (`r + r`)
- `Triad` enum for classification with three variants:
  - `Antifragile` - benefits from volatility (convex response)
  - `Fragile` - harmed by volatility (concave response)
  - `Robust` - unaffected by volatility (linear response)
- `TriadAnalysis` extension trait with methods:
  - `classify()` - classify system using Taleb's convexity test
  - `is_antifragile()` - check if system is antifragile
  - `gains_from_stress()` - check if higher stress leads to better payoff
  - `is_stable()` - check if payoff is stable across stress levels
- `Verified<T>` wrapper for systems with verified Triad classification
- `InvalidTriadValue` error type for `TryFrom<u8>` conversion failures
- `ParseTriadError` error type for `FromStr` parsing failures
- Trait implementations for `Triad`:
  - `Ord` / `PartialOrd` - ordering by desirability (Fragile < Robust < Antifragile)
  - `From<Triad> for u8` - convert to numeric value
  - `TryFrom<u8> for Triad` - convert from numeric value
  - `FromStr` - parse from string (case insensitive)
  - `From<Triad> for &'static str` - convert to string
  - `Display` - human-readable format
  - `Default` - defaults to `Robust`
- `no_std` support (disable `std` feature)
- Optional `serde` support for `Triad` and `Verified`
