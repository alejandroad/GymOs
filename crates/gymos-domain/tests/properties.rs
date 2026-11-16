/*!
 * Property tests for the exact e1RM comparison (DD-13, DD-09).
 *
 * The verdict relies on `E1rm::cmp_exact` being a well-behaved order that
 * agrees with the numbers the user sees, across both units and every
 * effort scale.
 */

#![expect(
    clippy::unwrap_used,
    reason = "strategies only produce values the constructors accept"
)]

use core::cmp::Ordering;

use gymos_domain::progression::E1rm;
use gymos_domain::{Effort, Load, Reps, Rir, Rpe, WeightUnit};
use proptest::prelude::*;

fn load() -> impl Strategy<Value = Load> {
    prop_oneof![
        (0_u32..=100_000).prop_map(|h| Load::new(h, WeightUnit::Kilograms).unwrap()),
        (0_u32..=220_462).prop_map(|h| Load::new(h, WeightUnit::Pounds).unwrap()),
    ]
}

fn effort() -> impl Strategy<Value = Option<Effort>> {
    prop_oneof![
        Just(None),
        (2_u8..=20).prop_map(|h| Some(Effort::Rpe(Rpe::from_half_points(h).unwrap()))),
        (0_u8..=10).prop_map(|n| Some(Effort::Rir(Rir::new(n).unwrap()))),
    ]
}

/** Only sets within the rep cap have an e1RM. */
fn e1rm() -> impl Strategy<Value = E1rm> {
    (load(), 1_u16..=12, effort()).prop_filter_map("above the rep cap", |(load, reps, effort)| {
        E1rm::of_set(load, Reps::new(reps).unwrap(), effort)
    })
}

proptest! {
    #[test]
    fn comparison_is_antisymmetric(a in e1rm(), b in e1rm()) {
        prop_assert_eq!(a.cmp_exact(b), b.cmp_exact(a).reverse());
    }

    #[test]
    fn comparison_is_transitive(a in e1rm(), b in e1rm(), c in e1rm()) {
        if a.cmp_exact(b) != Ordering::Greater && b.cmp_exact(c) != Ordering::Greater {
            prop_assert_ne!(a.cmp_exact(c), Ordering::Greater);
        }
        if a.cmp_exact(b) == Ordering::Equal && b.cmp_exact(c) == Ordering::Equal {
            prop_assert_eq!(a.cmp_exact(c), Ordering::Equal);
        }
    }

    #[test]
    fn exact_order_agrees_with_the_displayed_kilograms(a in e1rm(), b in e1rm()) {
        let (x, y) = (a.kilograms(), b.kilograms());
        if (x - y).abs() > 1e-9 {
            prop_assert_eq!(Some(a.cmp_exact(b)), x.partial_cmp(&y));
        }
    }

    #[test]
    fn one_more_rep_at_the_same_load_never_lowers_the_e1rm(
        load in load(),
        reps in 1_u16..=999,
        effort in effort(),
    ) {
        let fewer = E1rm::of_set(load, Reps::new(reps).unwrap(), effort);
        let more = E1rm::of_set(load, Reps::new(reps + 1).unwrap(), effort);
        /* Past the rep cap there is no estimate to compare. */
        if let (Some(fewer), Some(more)) = (fewer, more) {
            prop_assert_ne!(more.cmp_exact(fewer), Ordering::Less);
        }
    }
}
