//! Progress of one report computation: the phase and counters a progress readout polls.
//!
//! Compute code advances these lock-free counters as it goes; it never draws
//! anything. The renderer (`crate::render::progress`) samples them from its
//! own thread and turns them into a self-overwriting status line.

use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};

use tracing::trace;

/// The stage a report computation is in, in the order a refresh runs them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum Phase {
    /// Nothing started yet.
    #[default]
    Idle = 0,
    /// Walking the tree to find files; the total is not known yet.
    Walking = 1,
    /// Counting the lines of the files found.
    Counting = 2,
    /// Reading the last 30 days of git history.
    History = 3,
    /// Reading the status of every worktree.
    Worktrees = 4,
    /// Writing the caches and snapshot back.
    Saving = 5,
    /// The computation is finished.
    Done = 6,
}

impl Phase {
    /// The phase stored as `n`; unknown values read as `Done`, which only stops a readout.
    fn from_u8(n: u8) -> Self {
        match n {
            0 => Self::Idle,
            1 => Self::Walking,
            2 => Self::Counting,
            3 => Self::History,
            4 => Self::Worktrees,
            5 => Self::Saving,
            _ => Self::Done,
        }
    }
}

/// One consistent-enough reading of the counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sample {
    /// The current phase.
    pub phase: Phase,
    /// Items finished in this phase.
    pub done: u64,
    /// Items this phase will process, when known up front.
    pub total: Option<u64>,
}

/// Shared progress counters; cheap to advance from many threads at once.
#[derive(Debug, Default)]
pub struct Progress {
    phase: AtomicU8,
    done: AtomicU64,
    /// `u64::MAX` means the total is unknown.
    total: AtomicU64,
}

impl Progress {
    /// Starts `phase` with no items done; `total` is the item count when it is known.
    pub fn begin(&self, phase: Phase, total: Option<u64>) {
        trace!(?phase, ?total, "progress phase");
        self.done.store(0, Ordering::Relaxed);
        self.total
            .store(total.unwrap_or(u64::MAX), Ordering::Relaxed);
        self.phase.store(phase as u8, Ordering::Release);
    }

    /// Records one more finished item in the current phase.
    pub fn tick(&self) {
        self.done.fetch_add(1, Ordering::Relaxed);
    }

    /// Reads the counters; a reading taken while a phase starts may mix the old and new phase for one frame.
    pub fn sample(&self) -> Sample {
        let phase = Phase::from_u8(self.phase.load(Ordering::Acquire));
        let total = self.total.load(Ordering::Relaxed);
        Sample {
            phase,
            done: self.done.load(Ordering::Relaxed),
            total: (total != u64::MAX).then_some(total),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_progress_is_idle() {
        let p = Progress::default();
        assert_eq!(p.sample().phase, Phase::Idle);
    }

    #[test]
    fn begin_resets_done_and_sets_the_total() {
        let p = Progress::default();
        p.begin(Phase::Walking, None);
        p.tick();
        p.tick();
        assert_eq!(
            p.sample(),
            Sample {
                phase: Phase::Walking,
                done: 2,
                total: None
            }
        );
        p.begin(Phase::Counting, Some(10));
        p.tick();
        assert_eq!(
            p.sample(),
            Sample {
                phase: Phase::Counting,
                done: 1,
                total: Some(10)
            }
        );
    }

    #[test]
    fn every_phase_round_trips() {
        for phase in [
            Phase::Idle,
            Phase::Walking,
            Phase::Counting,
            Phase::History,
            Phase::Worktrees,
            Phase::Saving,
            Phase::Done,
        ] {
            assert_eq!(Phase::from_u8(phase as u8), phase);
        }
    }
}
