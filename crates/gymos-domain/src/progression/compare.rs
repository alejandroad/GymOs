/*!
 * The one place where two overload quantities are compared (DD-13,
 * "Cross-unit same load").
 *
 * - **Same entry unit:** exact. A micro-plate (+0.25 kg on 100 kg, 0.25%)
 *   is real progress and must register.
 * - **Different entry units:** within 0.5% counts as equal. 100 kg and
 *   220.46 lb differ by about 1 g. Without this tolerance, switching units
 *   would read as a regression.
 *
 * The check is exact integer arithmetic: `|a − b| × 200 ≤ max(a, b)`. The
 * threshold is part of algorithm version 1; changing it means bumping
 * [`ALGORITHM_VERSION`](crate::ALGORITHM_VERSION).
 */

use core::cmp::Ordering;

use crate::units::Load;

/** 0.5% expressed as a denominator: `|a − b| × 200 ≤ max(a, b)`. */
const CROSS_UNIT_TOLERANCE_DENOMINATOR: u128 = 200;

/**
 * Compares two non-negative exact quantities (scaled masses, e1RMs,
 * volumes). `same_unit` says whether every load behind both values was
 * entered in one unit.
 */
pub(crate) fn compare_exact_or_tolerant(after: i128, before: i128, same_unit: bool) -> Ordering {
    if !same_unit && within_tolerance(after, before) {
        return Ordering::Equal;
    }
    after.cmp(&before)
}

/** Compares two loads under the same rule. */
pub(crate) fn compare_loads(after: Load, before: Load) -> Ordering {
    compare_exact_or_tolerant(
        i128::from(after.mass().nanograms()),
        i128::from(before.mass().nanograms()),
        after.unit() == before.unit(),
    )
}

fn within_tolerance(a: i128, b: i128) -> bool {
    /* The products are bounded by roughly 1e38 / 200, far above any real input. */
    a.abs_diff(b)
        .saturating_mul(CROSS_UNIT_TOLERANCE_DENOMINATOR)
        <= a.max(b).unsigned_abs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::WeightUnit::{Kilograms, Pounds};

    fn kg(hundredths: u32) -> Load {
        Load::new(hundredths, Kilograms).unwrap()
    }

    fn lb(hundredths: u32) -> Load {
        Load::new(hundredths, Pounds).unwrap()
    }

    #[test]
    fn same_unit_is_exact_so_micro_plates_count() {
        assert_eq!(compare_loads(kg(10_025), kg(10_000)), Ordering::Greater);
        assert_eq!(compare_loads(kg(10_000), kg(10_000)), Ordering::Equal);
    }

    #[test]
    fn a_unit_switch_within_half_a_percent_is_the_same_load() {
        /* 220.46 lb ≈ 99.9997 kg. */
        assert_eq!(compare_loads(lb(22_046), kg(10_000)), Ordering::Equal);
        /* 221.55 lb ≈ 100.494 kg: still within 0.5%. */
        assert_eq!(compare_loads(lb(22_155), kg(10_000)), Ordering::Equal);
    }

    #[test]
    fn a_unit_switch_beyond_half_a_percent_is_a_change() {
        /* 222 lb ≈ 100.698 kg. */
        assert_eq!(compare_loads(lb(22_200), kg(10_000)), Ordering::Greater);
        assert_eq!(compare_loads(kg(10_000), lb(22_200)), Ordering::Less);
    }

    #[test]
    fn the_boundary_is_inclusive() {
        assert_eq!(
            compare_exact_or_tolerant(1_000, 995, false),
            Ordering::Equal
        );
        assert_eq!(
            compare_exact_or_tolerant(1_000, 994, false),
            Ordering::Greater
        );
        assert_eq!(compare_exact_or_tolerant(0, 0, false), Ordering::Equal);
    }
}
