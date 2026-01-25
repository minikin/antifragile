use antifragile::{Antifragile, Triad, TriadAnalysis, Verified};

// Example systems for testing
struct ConvexSystem;
struct ConcaveSystem;
struct LinearSystem;

impl Antifragile for ConvexSystem {
    type Stressor = f64;
    type Payoff = f64;

    fn payoff(&self, x: Self::Stressor) -> Self::Payoff {
        x * x
    }
}

impl Antifragile for ConcaveSystem {
    type Stressor = f64;
    type Payoff = f64;

    fn payoff(&self, x: Self::Stressor) -> Self::Payoff {
        x.abs().sqrt()
    }
}

impl Antifragile for LinearSystem {
    type Stressor = f64;
    type Payoff = f64;

    fn payoff(&self, x: Self::Stressor) -> Self::Payoff {
        2.0 * x + 5.0
    }
}

#[test]
fn test_end_to_end_workflow() {
    // 1. Create a system
    let system = ConvexSystem;

    // 2. Classify it
    let classification = system.classify(10.0, 1.0);
    assert_eq!(classification, Triad::Antifragile);

    // 3. Use helper methods
    assert!(system.is_antifragile(10.0, 1.0));
    assert!(system.gains_from_stress(1.0, 2.0));

    // 4. Wrap in Verified
    let verified = Verified::check(system, 10.0, 1.0);
    assert!(verified.is_antifragile());
    assert_eq!(verified.classification(), Triad::Antifragile);

    // 5. Access inner system through Deref
    assert!((verified.payoff(3.0) - 9.0).abs() < f64::EPSILON);
}

#[test]
fn test_all_triad_classifications() {
    let convex = ConvexSystem;
    let concave = ConcaveSystem;
    let linear = LinearSystem;

    assert_eq!(convex.classify(10.0, 1.0), Triad::Antifragile);
    assert_eq!(concave.classify(10.0, 1.0), Triad::Fragile);
    assert_eq!(linear.classify(10.0, 1.0), Triad::Robust);
}

#[test]
fn test_triad_ordering() {
    let mut classifications = vec![Triad::Antifragile, Triad::Fragile, Triad::Robust];
    classifications.sort();

    assert_eq!(
        classifications,
        vec![Triad::Fragile, Triad::Robust, Triad::Antifragile]
    );
}

#[test]
fn test_triad_conversions() {
    // To/from u8
    assert_eq!(u8::from(Triad::Antifragile), 0);
    assert_eq!(Triad::try_from(0u8), Ok(Triad::Antifragile));

    // To/from string
    let s: &str = Triad::Fragile.into();
    assert_eq!(s, "fragile");
    assert_eq!("fragile".parse::<Triad>(), Ok(Triad::Fragile));

    // Case insensitive parsing
    assert_eq!("ROBUST".parse::<Triad>(), Ok(Triad::Robust));
}

#[test]
fn test_verified_wrapper() {
    let verified = Verified::check(ConvexSystem, 10.0, 1.0);

    assert!(verified.is_antifragile());
    assert!(!verified.is_fragile());
    assert!(!verified.is_robust());

    // Access inner via AsRef
    let inner: &ConvexSystem = verified.as_ref();
    assert!((inner.payoff(2.0) - 4.0).abs() < f64::EPSILON);

    // Unwrap
    let _system = verified.into_inner();
}

#[cfg(feature = "serde")]
#[test]
fn test_serde_triad_roundtrip() {
    let triad = Triad::Antifragile;
    let json = serde_json::to_string(&triad).unwrap();
    let parsed: Triad = serde_json::from_str(&json).unwrap();
    assert_eq!(triad, parsed);

    // Test all variants
    for variant in [Triad::Antifragile, Triad::Fragile, Triad::Robust] {
        let json = serde_json::to_string(&variant).unwrap();
        let parsed: Triad = serde_json::from_str(&json).unwrap();
        assert_eq!(variant, parsed);
    }
}

#[cfg(feature = "serde")]
#[test]
fn test_serde_verified_roundtrip() {
    #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
    struct SerializableSystem {
        multiplier: f64,
    }

    impl Antifragile for SerializableSystem {
        type Stressor = f64;
        type Payoff = f64;

        fn payoff(&self, x: Self::Stressor) -> Self::Payoff {
            self.multiplier * x * x
        }
    }

    let system = SerializableSystem { multiplier: 2.0 };
    let verified = Verified::check(system, 10.0, 1.0);

    let json = serde_json::to_string(&verified).unwrap();
    let parsed: Verified<SerializableSystem> = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.classification(), Triad::Antifragile);
    assert!((parsed.inner().multiplier - 2.0).abs() < f64::EPSILON);
}
