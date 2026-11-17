/*!
 * Overload signals: each one is a single, explainable change since last
 * time (DD-13).
 *
 * A signal carries its before and after values, so the UI can render
 * "+1 rep at 100 kg on set 2 (8→9)" without recomputing anything. Signals
 * are listed strongest first, in the order of the [`Signal`] variants. Within
 * one kind, they are ordered by set.
 *
 * Only `WeightReps` is detected for now. Other tracking modes will follow
 * the same pattern with fewer signals.
 */

use core::cmp::Ordering;

use super::compare::{compare_exact_or_tolerant, compare_loads};
use super::e1rm::E1rm;
use super::pairing::{Pairing, is_pairable, pair_sets};
use super::scheme::ProgressionScheme;
use crate::effort::Effort;
use crate::set::SetType;
use crate::units::{Load, Reps};

/** Which way a signal moved. */
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Direction {
    /** Better than last time. */
    Up,
    /** Worse than last time. */
    Down,
}

impl Direction {
    /** `after` compared with `before`: `Greater` is up. `Equal` is no signal. */
    const fn of(ordering: Ordering) -> Option<Self> {
        match ordering {
            Ordering::Greater => Some(Self::Up),
            Ordering::Less => Some(Self::Down),
            Ordering::Equal => None,
        }
    }
}

/**
 * Volume load: Σ load × reps, exactly, in nanogram-reps.
 *
 * `i128` can't overflow: one set is at most 1e15 ng × 1000 reps.
 */
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Volume(i128);

impl Volume {
    /** The exact value in nanogram-reps. */
    pub const fn nanogram_reps(self) -> i128 {
        self.0
    }

    /** The value in kilogram-reps, for display only. */
    pub fn kilogram_reps(self) -> f64 {
        #[expect(
            clippy::cast_precision_loss,
            reason = "display only; comparisons use the exact integer"
        )]
        let nanogram_reps = self.0 as f64;
        nanogram_reps / 1e12
    }
}

/**
 * One change since the baseline, strongest kind first.
 *
 * `set_index` is the 0-based position among the current session's pairable
 * sets (see [`pair_sets`](super::pair_sets)). The UI shows it 1-based.
 */
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Signal {
    /** The best set's estimated 1RM changed. */
    E1rm {
        /** Which way it moved. */
        direction: Direction,
        /** Last time's best e1RM. */
        before: E1rm,
        /** This time's best e1RM. */
        after: E1rm,
    },
    /**
     * Double progression: the top load went up and the reps at that load
     * are still within the scheme's range. Always positive.
     */
    LoadStep {
        /** Last time's top load. */
        before: Load,
        /** This time's top load. */
        after: Load,
        /** The fewest reps done at the new top load. */
        reps: Reps,
    },
    /** A paired set did the same reps with a different load. */
    LoadAtReps {
        /** Which way the load moved. */
        direction: Direction,
        /** The current set's position among pairable sets (0-based). */
        set_index: usize,
        /** The reps both sets did. */
        reps: Reps,
        /** Last time's load. */
        before: Load,
        /** This time's load. */
        after: Load,
    },
    /** A paired set did different reps at the same load. */
    RepsAtLoad {
        /** Which way the reps moved. */
        direction: Direction,
        /** The current set's position among pairable sets (0-based). */
        set_index: usize,
        /** This time's load. */
        load: Load,
        /** Last time's reps. */
        before: Reps,
        /** This time's reps. */
        after: Reps,
    },
    /** More or fewer pairable sets, every paired set unchanged. */
    WorkingSets {
        /** Which way the count moved. */
        direction: Direction,
        /** Last time's pairable sets. */
        before: usize,
        /** This time's pairable sets. */
        after: usize,
    },
    /**
     * Σ load × reps over counted sets changed. Up only when the e1RM did
     * not drop, so piling on junk volume doesn't read as progress. If only
     * one side has an e1RM (the other is above the rep cap), a drop can't be
     * ruled out, so there is no `Up`. `Down` is always listed.
     */
    VolumeLoad {
        /** Which way the volume moved. */
        direction: Direction,
        /** Last time's volume. */
        before: Volume,
        /** This time's volume. */
        after: Volume,
    },
    /** The same load and reps on a paired set, at a different effort. */
    Effort {
        /** Up when it felt easier. */
        direction: Direction,
        /** The current set's position among pairable sets (0-based). */
        set_index: usize,
        /** Last time's effort. */
        before: Effort,
        /** This time's effort. */
        after: Effort,
    },
}

impl Signal {
    /** Which way this signal points. A `LoadStep` is always `Up`. */
    pub const fn direction(&self) -> Direction {
        match *self {
            Self::LoadStep { .. } => Direction::Up,
            Self::E1rm { direction, .. }
            | Self::LoadAtReps { direction, .. }
            | Self::RepsAtLoad { direction, .. }
            | Self::WorkingSets { direction, .. }
            | Self::VolumeLoad { direction, .. }
            | Self::Effort { direction, .. } => direction,
        }
    }
}

/** A rule that could not be evaluated, listed so the explanation stays honest. */
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Skipped {
    /**
     * The e1RM was not compared: one side has counted sets, but every one is
     * above [`E1RM_REP_CAP`](super::E1RM_REP_CAP) effective reps.
     */
    E1rmAboveRepCap,
}

/** A counted weight × reps set, as the rules read it. */
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WeightRepsSet {
    pub(crate) set_type: SetType,
    pub(crate) load: Load,
    pub(crate) reps: Reps,
    pub(crate) effort: Option<Effort>,
}

/** What detection found. */
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Detection {
    pub(crate) signals: Vec<Signal>,
    pub(crate) skipped: Vec<Skipped>,
}

/**
 * Detects every `WeightReps` signal between two sessions' sets. Warm-ups
 * are dropped here. If either side has no counted sets, nothing is
 * comparable and the detection is empty.
 */
pub(crate) fn detect_weight_reps(
    baseline: &[WeightRepsSet],
    current: &[WeightRepsSet],
    scheme: Option<&ProgressionScheme>,
) -> Detection {
    let counted = |sets: &[WeightRepsSet]| -> Vec<WeightRepsSet> {
        sets.iter()
            .copied()
            .filter(|s| s.set_type.counts_by_default())
            .collect()
    };
    let (baseline, current) = (counted(baseline), counted(current));
    if baseline.is_empty() || current.is_empty() {
        return Detection::default();
    }

    let mut detection = Detection::default();
    let (before, after) = (best_e1rm(&baseline), best_e1rm(&current));
    /*
     * Volume may only rise as progress when the e1RM is known not to have
     * dropped. With both sides above the rep cap, e1RM says nothing either
     * way (example K). With only one side above it, a drop can't be ruled
     * out, so extra volume is not progress.
     */
    let volume_up_allowed = match (before, after) {
        (Some(before), Some(after)) => {
            let same_unit = before.load().unit() == after.load().unit();
            let ordering = compare_exact_or_tolerant(after.scaled(), before.scaled(), same_unit);
            if let Some(direction) = Direction::of(ordering) {
                detection.signals.push(Signal::E1rm {
                    direction,
                    before,
                    after,
                });
            }
            ordering != Ordering::Less
        }
        (None, None) => {
            detection.skipped.push(Skipped::E1rmAboveRepCap);
            true
        }
        (Some(_), None) | (None, Some(_)) => {
            detection.skipped.push(Skipped::E1rmAboveRepCap);
            false
        }
    };

    detection
        .signals
        .extend(load_step(&baseline, &current, scheme));
    detection
        .signals
        .extend(paired_signals(&baseline, &current));
    detection
        .signals
        .extend(volume_load(&baseline, &current, volume_up_allowed));
    detection.signals.sort_by_key(kind_rank);
    detection
}

/** Output order: strongest kind first. The sort is stable, so sets stay in order. */
const fn kind_rank(signal: &Signal) -> u8 {
    match signal {
        Signal::E1rm { .. } => 0,
        Signal::LoadStep { .. } => 1,
        Signal::LoadAtReps { .. } => 2,
        Signal::RepsAtLoad { .. } => 3,
        Signal::WorkingSets { .. } => 4,
        Signal::VolumeLoad { .. } => 5,
        Signal::Effort { .. } => 6,
    }
}

fn best_e1rm(sets: &[WeightRepsSet]) -> Option<E1rm> {
    sets.iter()
        .filter_map(|s| E1rm::of_set(s.load, s.reps, s.effort))
        .max_by(|a, b| a.cmp_exact(*b))
}

fn top_pairable_load(sets: &[WeightRepsSet]) -> Option<Load> {
    sets.iter()
        .filter(|s| is_pairable(s.set_type))
        .map(|s| s.load)
        .max_by_key(|load| load.mass())
}

fn load_step(
    baseline: &[WeightRepsSet],
    current: &[WeightRepsSet],
    scheme: Option<&ProgressionScheme>,
) -> Option<Signal> {
    let scheme = scheme?;
    let before = top_pairable_load(baseline)?;
    let after = top_pairable_load(current)?;
    if compare_loads(after, before) != Ordering::Greater {
        return None;
    }
    let reps = current
        .iter()
        .filter(|s| is_pairable(s.set_type) && compare_loads(s.load, after) == Ordering::Equal)
        .map(|s| s.reps)
        .min()?;
    (reps >= scheme.reps().min()).then_some(Signal::LoadStep {
        before,
        after,
        reps,
    })
}

/** How hard a set felt, on the RPE half-point scale. RIR 1 is RPE 9. */
fn difficulty(effort: Effort) -> u16 {
    match effort {
        Effort::Rpe(rpe) => u16::from(rpe.half_points()),
        Effort::Rir(rir) => 20_u16.saturating_sub(2 * u16::from(rir.get())),
    }
}

/** `LoadAtReps`, `RepsAtLoad`, `WorkingSets` and `Effort`: the signals read off pairs. */
fn paired_signals(baseline: &[WeightRepsSet], current: &[WeightRepsSet]) -> Vec<Signal> {
    let types = |sets: &[WeightRepsSet]| sets.iter().map(|s| s.set_type).collect::<Vec<_>>();
    let pairing = pair_sets(&types(baseline), &types(current));
    let mut signals = Vec::new();
    let mut every_pair_identical = true;

    for pair in &pairing.pairs {
        let (Some(before), Some(after)) = (baseline.get(pair.baseline), current.get(pair.current))
        else {
            continue;
        };
        let set_index = pair.set_index;
        let load = compare_loads(after.load, before.load);
        let reps = after.reps.cmp(&before.reps);
        every_pair_identical &= load == Ordering::Equal && reps == Ordering::Equal;

        match (Direction::of(load), Direction::of(reps)) {
            (Some(direction), None) => signals.push(Signal::LoadAtReps {
                direction,
                set_index,
                reps: after.reps,
                before: before.load,
                after: after.load,
            }),
            (None, Some(direction)) => signals.push(Signal::RepsAtLoad {
                direction,
                set_index,
                load: after.load,
                before: before.reps,
                after: after.reps,
            }),
            (None, None) => {
                if let (Some(was), Some(now)) = (before.effort, after.effort) {
                    /* Easier is better: compare before against after. */
                    if let Some(direction) = Direction::of(difficulty(was).cmp(&difficulty(now))) {
                        signals.push(Signal::Effort {
                            direction,
                            set_index,
                            before: was,
                            after: now,
                        });
                    }
                }
            }
            /* Load and reps both changed: e1RM and volume speak for it. */
            (Some(_), Some(_)) => {}
        }
    }

    /* At least one pair, or "every pair identical" would be vacuously true. */
    if every_pair_identical
        && !pairing.pairs.is_empty()
        && extra_sets_repeat_paired_work(baseline, current, &pairing)
    {
        let count = pairing.current_pairable.cmp(&pairing.baseline_pairable);
        if let Some(direction) = Direction::of(count) {
            signals.push(Signal::WorkingSets {
                direction,
                before: pairing.baseline_pairable,
                after: pairing.current_pairable,
            });
        }
    }
    signals
}

/**
 * Whether every unpaired pairable set on the longer side repeats work from
 * the other side: the same reps at the same load as one of its paired sets.
 * `WorkingSets` means "more or fewer sets at the same load × reps" (product
 * §3.3), so an extra set at another load or rep count is not one.
 */
fn extra_sets_repeat_paired_work(
    baseline: &[WeightRepsSet],
    current: &[WeightRepsSet],
    pairing: &Pairing,
) -> bool {
    let (longer, shorter, longer_paired, shorter_paired): (_, _, Vec<usize>, Vec<usize>) =
        if pairing.current_pairable >= pairing.baseline_pairable {
            (
                current,
                baseline,
                pairing.pairs.iter().map(|p| p.current).collect(),
                pairing.pairs.iter().map(|p| p.baseline).collect(),
            )
        } else {
            (
                baseline,
                current,
                pairing.pairs.iter().map(|p| p.baseline).collect(),
                pairing.pairs.iter().map(|p| p.current).collect(),
            )
        };
    let same_work = |a: &WeightRepsSet, b: &WeightRepsSet| {
        a.reps == b.reps && compare_loads(a.load, b.load) == Ordering::Equal
    };
    longer
        .iter()
        .enumerate()
        .filter(|&(i, s)| is_pairable(s.set_type) && !longer_paired.contains(&i))
        .all(|(_, extra)| {
            shorter_paired
                .iter()
                .filter_map(|&i| shorter.get(i))
                .any(|paired| same_work(extra, paired))
        })
}

fn volume(sets: &[WeightRepsSet]) -> Volume {
    Volume(
        sets.iter()
            .map(|s| i128::from(s.load.mass().nanograms()) * i128::from(s.reps.get()))
            .sum(),
    )
}

fn volume_load(
    baseline: &[WeightRepsSet],
    current: &[WeightRepsSet],
    up_allowed: bool,
) -> Option<Signal> {
    let (before, after) = (volume(baseline), volume(current));
    let mut units = baseline.iter().chain(current).map(|s| s.load.unit());
    let first = units.next()?;
    let same_unit = units.all(|unit| unit == first);
    let direction = Direction::of(compare_exact_or_tolerant(after.0, before.0, same_unit))?;
    (direction == Direction::Down || up_allowed).then_some(Signal::VolumeLoad {
        direction,
        before,
        after,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effort::{Rir, Rpe};
    use crate::progression::scheme::RepRange;
    use crate::units::WeightUnit::Kilograms;

    fn set(set_type: SetType, hundredths: u32, reps: u16) -> WeightRepsSet {
        WeightRepsSet {
            set_type,
            load: Load::new(hundredths, Kilograms).unwrap(),
            reps: Reps::new(reps).unwrap(),
            effort: None,
        }
    }

    fn working(hundredths: u32, reps: u16) -> WeightRepsSet {
        set(SetType::Working, hundredths, reps)
    }

    fn scheme(min: u16, max: u16) -> ProgressionScheme {
        let range = RepRange::new(Reps::new(min).unwrap(), Reps::new(max).unwrap()).unwrap();
        ProgressionScheme::new(range, Load::new(250, Kilograms).unwrap()).unwrap()
    }

    fn kinds(d: &Detection) -> Vec<u8> {
        d.signals.iter().map(kind_rank).collect()
    }

    #[test]
    fn nothing_is_comparable_without_counted_sets_on_both_sides() {
        let warmup = [set(SetType::Warmup, 6_000, 10)];
        assert_eq!(
            detect_weight_reps(&warmup, &[working(10_000, 8)], None),
            Detection::default()
        );
        assert_eq!(
            detect_weight_reps(&[working(10_000, 8)], &[], None),
            Detection::default()
        );
    }

    #[test]
    fn load_step_needs_the_scheme_minimum_at_the_new_load() {
        let baseline = [working(10_000, 8); 2];
        let current = [working(10_250, 6), working(10_250, 5)];
        /* 5 reps at the new load is below a 6–8 range: no LoadStep. */
        let d = detect_weight_reps(&baseline, &current, Some(&scheme(6, 8)));
        assert!(!kinds(&d).contains(&1));
        /* In a 5–8 range it is. */
        let d = detect_weight_reps(&baseline, &current, Some(&scheme(5, 8)));
        assert!(d.signals.contains(&Signal::LoadStep {
            before: baseline[0].load,
            after: current[0].load,
            reps: current[1].reps,
        }));
    }

    #[test]
    fn working_sets_need_every_pair_identical() {
        let baseline = [working(10_000, 8); 2];
        let current = [working(10_000, 8), working(10_000, 7), working(10_000, 8)];
        let d = detect_weight_reps(&baseline, &current, None);
        assert!(!kinds(&d).contains(&4));
    }

    #[test]
    fn extra_sets_must_repeat_last_times_load_and_reps() {
        let baseline = [working(10_000, 8); 3];
        /* An extra 0 kg × 1 set is not "one more set at 100 kg × 8". */
        let mut current = vec![working(10_000, 8); 3];
        current.push(working(0, 1));
        assert_eq!(
            detect_weight_reps(&baseline, &current, None),
            Detection::default()
        );
        /* An extra set at a micro-plate heavier load isn't either. */
        current[3] = working(10_025, 8);
        assert!(!kinds(&detect_weight_reps(&baseline, &current, None)).contains(&4));
        /* One more identical set is. */
        current[3] = working(10_000, 8);
        assert!(kinds(&detect_weight_reps(&baseline, &current, None)).contains(&4));
        /* The same holds for a dropped set: the missing one must match too. */
        let mut longer = vec![working(10_000, 8); 3];
        longer.push(working(6_000, 12));
        let d = detect_weight_reps(&longer, &baseline, None);
        assert!(!kinds(&d).contains(&4));
    }

    #[test]
    fn volume_is_not_progress_when_only_one_side_has_an_e1rm() {
        let baseline = [working(10_000, 8); 3];
        let current = [working(6_000, 15); 3];
        let d = detect_weight_reps(&baseline, &current, None);
        /* 2400 → 2700 kg·reps, but the heavy e1RM may well have dropped. */
        assert_eq!(d.signals, []);
        assert_eq!(d.skipped, [Skipped::E1rmAboveRepCap]);
        /* A volume drop is still listed. */
        let current = [working(6_000, 15)];
        let d = detect_weight_reps(&baseline, &current, None);
        assert_eq!(kinds(&d), [5]);
        assert_eq!(d.signals[0].direction(), Direction::Down);
    }

    #[test]
    fn working_sets_need_at_least_one_pair() {
        let baseline = [set(SetType::Backoff, 9_000, 8)];
        let current = [working(10_000, 8)];
        let d = detect_weight_reps(&baseline, &current, None);
        assert!(!kinds(&d).contains(&4));
    }

    #[test]
    fn extra_volume_is_not_progress_when_the_e1rm_drops() {
        let baseline = [working(10_000, 5)];
        let current = [working(9_000, 6), working(9_000, 6)];
        let d = detect_weight_reps(&baseline, &current, None);
        assert_eq!(d.signals.len(), 1);
        assert_eq!(d.signals[0].direction(), Direction::Down);
        assert!(matches!(d.signals[0], Signal::E1rm { .. }));
    }

    #[test]
    fn effort_compares_rpe_and_rir_on_one_scale() {
        let mut before = working(10_000, 8);
        let mut after = before;
        before.effort = Some(Effort::Rpe(Rpe::from_half_points(18).unwrap()));
        after.effort = Some(Effort::Rir(Rir::new(1).unwrap()));
        let d = detect_weight_reps(&[before], &[after], None);
        assert_eq!(d.signals, []);
        after.effort = Some(Effort::Rir(Rir::new(2).unwrap()));
        let d = detect_weight_reps(&[before], &[after], None);
        assert_eq!(kinds(&d), [0, 6]);
        assert!(d.signals.iter().all(|s| s.direction() == Direction::Up));
    }
}
