/*!
 * Set pairing: which set this session is compared with which set last time
 * (DD-13 gap 3).
 *
 * - **Pairable:** `TopSet`, `Working` and `Failure` sets.
 * - **`TopSet`s pair first:** the n-th top set with the n-th top set, so
 *   "top set 100 → 102.5 kg" compares like with like even when the number
 *   of sets around it changes.
 * - **The rest pair by order:** every pairable set still unpaired,
 *   including extra top sets. A session with no top set is plain order.
 * - **Unequal counts:** extra sets stay unpaired.
 *
 * `Drop` and `Backoff` sets are never paired (they still count toward
 * volume), and warm-ups are excluded everywhere.
 *
 * Pairing only looks at set types, so it works for every tracking mode.
 */

use crate::set::SetType;

/** Whether sets of this type are compared one-to-one with last time. */
pub const fn is_pairable(set_type: SetType) -> bool {
    match set_type {
        SetType::TopSet | SetType::Working | SetType::Failure => true,
        SetType::Warmup | SetType::Drop | SetType::Backoff => false,
    }
}

/** One baseline set matched with one current set. */
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Pair {
    /** Index of the baseline set in the slice given to [`pair_sets`]. */
    pub baseline: usize,
    /** Index of the current set in the slice given to [`pair_sets`]. */
    pub current: usize,
    /**
     * The current set's 0-based position among the current session's
     * pairable sets. This is the set number signals report (the UI adds 1).
     */
    pub set_index: usize,
}

/** The result of pairing two sessions' sets. */
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pairing {
    /** The pairs, ordered by the current set. */
    pub pairs: Vec<Pair>,
    /** How many baseline sets were pairable. */
    pub baseline_pairable: usize,
    /** How many current sets were pairable. */
    pub current_pairable: usize,
}

/**
 * Pairs `baseline` sets with `current` sets, given each set's type in
 * performed order.
 */
pub fn pair_sets(baseline: &[SetType], current: &[SetType]) -> Pairing {
    let indices = |types: &[SetType], keep: fn(SetType) -> bool| -> Vec<usize> {
        types
            .iter()
            .enumerate()
            .filter(|&(_, &t)| keep(t))
            .map(|(i, _)| i)
            .collect()
    };
    let is_top = |t: SetType| t == SetType::TopSet;

    let current_pairable = indices(current, is_pairable);
    let top: Vec<(usize, usize)> = indices(baseline, is_top)
        .into_iter()
        .zip(indices(current, is_top))
        .collect();

    let unpaired = |types: &[SetType], taken: &[usize]| -> Vec<usize> {
        indices(types, is_pairable)
            .into_iter()
            .filter(|i| !taken.contains(i))
            .collect()
    };
    let taken_baseline: Vec<usize> = top.iter().map(|&(b, _)| b).collect();
    let taken_current: Vec<usize> = top.iter().map(|&(_, c)| c).collect();
    let rest = unpaired(baseline, &taken_baseline)
        .into_iter()
        .zip(unpaired(current, &taken_current));

    let mut pairs: Vec<Pair> = top
        .into_iter()
        .chain(rest)
        .filter_map(|(baseline, current)| {
            let set_index = current_pairable.iter().position(|&i| i == current)?;
            Some(Pair {
                baseline,
                current,
                set_index,
            })
        })
        .collect();
    pairs.sort_unstable_by_key(|pair| pair.current);

    Pairing {
        pairs,
        baseline_pairable: baseline.iter().filter(|&&t| is_pairable(t)).count(),
        current_pairable: current_pairable.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::set::SetType::{Backoff, Drop, Failure, TopSet, Warmup, Working};

    fn pairs(pairing: &Pairing) -> Vec<(usize, usize, usize)> {
        pairing
            .pairs
            .iter()
            .map(|p| (p.baseline, p.current, p.set_index))
            .collect()
    }

    #[test]
    fn pairs_working_sets_by_order() {
        let p = pair_sets(&[Working, Working, Working], &[Working, Working, Working]);
        assert_eq!(pairs(&p), [(0, 0, 0), (1, 1, 1), (2, 2, 2)]);
        assert_eq!((p.baseline_pairable, p.current_pairable), (3, 3));
    }

    #[test]
    fn ignores_warmups_drops_and_backoffs() {
        let p = pair_sets(
            &[Warmup, Working, Drop, Failure],
            &[Warmup, Warmup, Working, Backoff, Failure],
        );
        assert_eq!(pairs(&p), [(1, 2, 0), (3, 4, 1)]);
        assert_eq!((p.baseline_pairable, p.current_pairable), (2, 2));
    }

    #[test]
    fn pairs_top_sets_first_even_when_sets_before_them_change() {
        /* An extra working set before the top set must not shift it. */
        let p = pair_sets(&[TopSet, Working], &[Working, TopSet, Working]);
        assert_eq!(pairs(&p), [(1, 0, 0), (0, 1, 1)]);
    }

    #[test]
    fn falls_back_to_order_without_a_top_set() {
        let p = pair_sets(&[TopSet, Working], &[Working, Working]);
        assert_eq!(pairs(&p), [(0, 0, 0), (1, 1, 1)]);
    }

    #[test]
    fn leaves_extra_sets_unpaired() {
        let p = pair_sets(&[Working; 3], &[Working; 4]);
        assert_eq!(pairs(&p), [(0, 0, 0), (1, 1, 1), (2, 2, 2)]);
        assert_eq!((p.baseline_pairable, p.current_pairable), (3, 4));
        let p = pair_sets(&[TopSet, TopSet], &[TopSet]);
        assert_eq!(pairs(&p), [(0, 0, 0)]);
    }

    #[test]
    fn pairs_nothing_when_either_side_is_empty() {
        assert_eq!(pair_sets(&[], &[Working]).pairs, []);
        assert_eq!(pair_sets(&[Backoff], &[Working]).pairs, []);
    }
}
