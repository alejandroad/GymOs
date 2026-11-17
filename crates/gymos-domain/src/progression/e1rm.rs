/*!
 * Estimated one-rep max (e1RM): Epley, compared exactly (DD-13).
 *
 * Epley is `w × (1 + r/30)`, so e1RM is `w × k(r) / 30` with `k(r) = 30 + r`.
 * Because the effort adjustment works in half-reps, everything here is
 * scaled by two: `e1RM = w × f / 60`, where
 *
 * - `h = 2 × r_eff` is the effective reps in half-reps, and
 * - `f = 60 + h`, except that a true single (`h = 2`) gives `f = 60`: a
 *   single rep *is* the 1RM, so it must not beat itself.
 *
 * Two e1RMs are compared by cross-multiplying `mass_ng × f` in `i128`. The
 * `/ 60` is shared, so it cancels. Floats only appear in
 * [`E1rm::kilograms`], for display.
 */

use core::cmp::Ordering;

use crate::effort::Effort;
use crate::units::{Load, Reps};

/**
 * The most effective reps an e1RM is computed from. Past about 10–12 reps
 * the formulas lose accuracy, so such sets give no estimate (DD-13).
 */
pub const E1RM_REP_CAP: u16 = 12;

/** RPE 10 in half-points: no reps left in reserve. */
const RPE_MAX_HALF_POINTS: u16 = 20;
/** `30` in Epley's `k(r) = 30 + r`, scaled to half-reps. */
const EPLEY_BASE: u16 = 60;
/** A single rep at RPE 10 (or RIR 0), in half-reps. */
const TRUE_SINGLE: u16 = 2;

/**
 * An e1RM: the load and effective reps of the set it was estimated from.
 *
 * It keeps its inputs rather than a rounded number, so it stays exact and
 * can explain itself ("102.5 kg × 6 → 123.0 kg"). Two e1RMs from different
 * sets can be equally strong, so this type doesn't implement `Ord`. Compare
 * them with [`E1rm::cmp_exact`].
 */
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct E1rm {
    load: Load,
    half_reps: u16,
}

impl E1rm {
    /**
     * Estimates the e1RM of one weight × reps set.
     *
     * Effort adjusts the reps to "reps to failure":
     * - RPE: adds `10 − RPE` reps in reserve (RPE 8.5 adds 1.5).
     * - RIR: adds the reps in reserve.
     * - No effort logged: the reps are used as they are.
     *
     * Returns `None` when the effective reps exceed [`E1RM_REP_CAP`].
     */
    pub fn of_set(load: Load, reps: Reps, effort: Option<Effort>) -> Option<Self> {
        let reserve_half_reps = match effort {
            /* Rpe guarantees 2..=20 half-points, so this can't underflow. */
            Some(Effort::Rpe(rpe)) => RPE_MAX_HALF_POINTS - u16::from(rpe.half_points()),
            Some(Effort::Rir(rir)) => 2 * u16::from(rir.get()),
            None => 0,
        };
        /* Reps ≤ 1000 and reserve ≤ 20, so this stays far below u16::MAX. */
        let half_reps = 2 * reps.get() + reserve_half_reps;
        (half_reps <= 2 * E1RM_REP_CAP).then_some(Self { load, half_reps })
    }

    /** The load of the set the estimate came from. */
    pub const fn load(self) -> Load {
        self.load
    }

    /**
     * The effective reps (reps plus reps in reserve), in half-reps:
     * 8 reps at RPE 8.5 is `19`.
     */
    pub const fn effective_half_reps(self) -> u16 {
        self.half_reps
    }

    /** The Epley factor over 60: `e1RM = load × factor / 60`. */
    const fn factor(self) -> u16 {
        if self.half_reps == TRUE_SINGLE {
            EPLEY_BASE
        } else {
            EPLEY_BASE + self.half_reps
        }
    }

    /**
     * The exact value scaled by 60, in nanograms: `mass_ng × factor`. Two
     * scaled values compare exactly like the e1RMs they come from.
     */
    pub(crate) fn scaled(self) -> i128 {
        i128::from(self.load.mass().nanograms()) * i128::from(self.factor())
    }

    /** Compares two e1RMs exactly, in integers. */
    pub fn cmp_exact(self, other: Self) -> Ordering {
        self.scaled().cmp(&other.scaled())
    }

    /** The e1RM in kilograms, for display only. Never compare these. */
    pub fn kilograms(self) -> f64 {
        self.load.mass().kilograms() * f64::from(self.factor()) / f64::from(EPLEY_BASE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effort::{Rir, Rpe};
    use crate::units::WeightUnit::{Kilograms, Pounds};

    fn kg(hundredths: u32) -> Load {
        Load::new(hundredths, Kilograms).unwrap()
    }

    fn reps(n: u16) -> Reps {
        Reps::new(n).unwrap()
    }

    fn rpe(half_points: u8) -> Effort {
        Effort::Rpe(Rpe::from_half_points(half_points).unwrap())
    }

    fn display(e: E1rm) -> f64 {
        (e.kilograms() * 100.0).round() / 100.0
    }

    #[test]
    fn matches_epley() {
        let e = E1rm::of_set(kg(10_000), reps(8), None).unwrap();
        assert!((display(e) - 126.67).abs() < 1e-9);
        let e = E1rm::of_set(kg(10_250), reps(6), None).unwrap();
        assert!((display(e) - 123.0).abs() < 1e-9);
    }

    #[test]
    fn a_true_single_is_the_one_rep_max() {
        let e = E1rm::of_set(kg(14_000), reps(1), None).unwrap();
        assert!((e.kilograms() - 140.0).abs() < 1e-9);
        /* A single at RPE 9 had a rep left: 2 effective reps. */
        let e = E1rm::of_set(kg(14_000), reps(1), Some(rpe(18))).unwrap();
        assert!((display(e) - 149.33).abs() < 1e-9);
    }

    #[test]
    fn rpe_and_rir_add_reps_in_reserve() {
        let at_rpe_9 = E1rm::of_set(kg(10_000), reps(8), Some(rpe(18))).unwrap();
        let at_rir_1 =
            E1rm::of_set(kg(10_000), reps(8), Some(Effort::Rir(Rir::new(1).unwrap()))).unwrap();
        let nine_reps = E1rm::of_set(kg(10_000), reps(9), None).unwrap();
        assert_eq!(at_rpe_9.effective_half_reps(), 18);
        assert_eq!(at_rpe_9.cmp_exact(at_rir_1), Ordering::Equal);
        assert_eq!(at_rpe_9.cmp_exact(nine_reps), Ordering::Equal);
        assert!((display(at_rpe_9) - 130.0).abs() < 1e-9);
        /* RPE 8.5 is 1.5 reps in reserve. */
        let half = E1rm::of_set(kg(10_000), reps(8), Some(rpe(17))).unwrap();
        assert_eq!(half.effective_half_reps(), 19);
    }

    #[test]
    fn skips_sets_above_the_rep_cap() {
        assert!(E1rm::of_set(kg(6_000), reps(12), None).is_some());
        assert!(E1rm::of_set(kg(6_000), reps(13), None).is_none());
        /* 11 reps at RPE 8 is 13 to failure. */
        assert!(E1rm::of_set(kg(6_000), reps(11), Some(rpe(16))).is_none());
        /* 11 reps at RPE 9.5 is 11.5 to failure. */
        assert!(E1rm::of_set(kg(6_000), reps(11), Some(rpe(19))).is_some());
    }

    #[test]
    fn compares_exactly_across_units() {
        let kilos = E1rm::of_set(kg(10_000), reps(8), None).unwrap();
        let pounds = E1rm::of_set(Load::new(22_046, Pounds).unwrap(), reps(8), None).unwrap();
        /* 220.46 lb is about 1 g lighter than 100 kg; exact means exact. */
        assert_eq!(pounds.cmp_exact(kilos), Ordering::Less);
    }
}
