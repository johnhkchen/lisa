//! Whether a loop is short a pane, and whether it may make another one.
//!
//! `.lisa-layout.kdl` declares the coding panes and Zellij creates them once, at
//! `lisa loop`. The plugin has never created one since, so every pane the run
//! loses it keeps losing: `pane_count = max_threads * 2`, whose own comment is
//! *"extra idle panes absorb new tickets while finishing panes wind down"*,
//! silently becomes `max_threads * 1` with no spare at all. Measured on
//! `screen-design` on 2026-08-13, after two panes were lost to a refused probe:
//!
//! ```text
//! lisa-5            → 4 children    healthy
//! screen-design-4   → 2 children    running on half its panes
//! ```
//!
//! Everything that *decides* lives here, pure, so it can be tested without a
//! terminal. `lib.rs` holds the half that talks to Zellij.
//!
//! ## What is counted, and how often
//!
//! The count is [`Census`]: how many coding panes the layout declared against
//! how many the last `PaneUpdate` showed in the loop's own tab. Neither number
//! is recomputed from `max_threads` — the layout is written per-run and is the
//! only thing that knows what this run asked for, so it carries the number it
//! wrote (see the `agent_panes` key in `lisa loop`'s generated layout). A run
//! launched from a layout that never said stays [`Census::declared`] `None`, and
//! nothing is healed: a scheduler that does not know how many panes it should
//! have is not one that should be inventing them.
//!
//! It is checked on `PaneUpdate` and nowhere else. Zellij delivers that event
//! when the pane set changes — which is exactly and only when this answer can
//! change — so noticing costs one comparison on an event the plugin already
//! subscribes to. There is no poll, and a pane is lost far too rarely to earn
//! one.
//!
//! ## The bound
//!
//! A pane that dies the instant it is created must not spin. [`Budget`] allows
//! [`MAX_REGENERATIONS`] asks in any [`REGENERATION_WINDOW_SECS`] window; the
//! window slides, so a run that loses one pane an hour heals every time. Spend
//! the budget and the loop **gives up for the rest of the run**: it says so once
//! in the activity feed, keeps working on the panes it has, and refuses the
//! `rail` ask with the same sentence. Giving up is sticky on purpose — a budget
//! that re-arms on a timer is the spin it exists to prevent, only slower — and
//! restarting the loop is the way back.
//!
//! An ask that has been made and not yet answered by a pane appearing is
//! [`Outstanding`]. It holds the next ask off until the pane arrives or
//! [`ADOPTION_TIMEOUT_SECS`] passes, so one lost pane costs one ask and not one
//! per `PaneUpdate` in between.

/// How many replacement panes one run may ask for inside a single window.
pub(crate) const MAX_REGENERATIONS: usize = 3;

/// The window those asks are counted over, in seconds.
pub(crate) const REGENERATION_WINDOW_SECS: u64 = 600;

/// How long a requested pane has to appear before the ask is written off.
///
/// Generous next to the round trip it covers — `open_terminal` reaches the
/// Zellij server and the resulting `PaneUpdate` comes back within a frame — and
/// deliberately shorter than the poll interval is long, so a request that Zellij
/// dropped is charged to the budget rather than blocking every later one.
pub(crate) const ADOPTION_TIMEOUT_SECS: u64 = 10;

/// What the layout declared against what the manifest showed.
///
/// The default — nothing declared, nothing seen — is the state before the first
/// `PaneUpdate`, and it decides [`Decision::Undeclared`]: a scheduler that has
/// not looked yet heals nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Census {
    /// Coding panes this run's layout created, or `None` when the layout it was
    /// launched from never said.
    pub(crate) declared: Option<usize>,
    /// Coding panes visible in the loop's tab right now.
    pub(crate) present: usize,
}

impl Census {
    /// How many panes are missing, or `None` when the layout never said.
    pub(crate) fn missing(&self) -> Option<usize> {
        self.declared
            .map(|declared| declared.saturating_sub(self.present))
    }

    /// Whether the board has every pane it declared.
    ///
    /// A run with more panes than it declared is whole: an operator who opened a
    /// pane of their own in this tab has added one, and Lisa does not close
    /// panes to make a number match.
    pub(crate) fn is_whole(&self) -> bool {
        self.missing().is_none_or(|missing| missing == 0)
    }
}

/// An ask that has been made and not yet answered by a pane appearing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Outstanding {
    /// Unix seconds at which the pane was asked for.
    pub(crate) asked_at: u64,
    /// The pane that held focus when the ask went out, so it can be handed back
    /// its focus afterwards. `None` when nothing in the tab held focus.
    pub(crate) focus_before: Option<u32>,
}

impl Outstanding {
    /// Whether this ask has waited longer than a pane takes to appear.
    pub(crate) fn is_stale(&self, now: u64) -> bool {
        now.saturating_sub(self.asked_at) >= ADOPTION_TIMEOUT_SECS
    }
}

/// How many replacement panes this run has asked for, and when.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Budget {
    /// Unix seconds of each ask, oldest first.
    asks: Vec<u64>,
    /// Set once the budget has been spent inside one window. Sticky for the run.
    given_up: bool,
}

impl Budget {
    /// Whether another ask is allowed at `now`.
    pub(crate) fn has_room(&self, now: u64) -> bool {
        !self.given_up && self.recent(now) < MAX_REGENERATIONS
    }

    /// Whether this run has stopped trying.
    pub(crate) fn has_given_up(&self) -> bool {
        self.given_up
    }

    /// Record one ask at `now`, and give up if it was the last one available.
    ///
    /// Returns whether this ask exhausted the budget, so the caller can say so
    /// exactly once.
    pub(crate) fn spend(&mut self, now: u64) -> bool {
        self.asks.push(now);
        if self.recent(now) >= MAX_REGENERATIONS {
            self.given_up = true;
            return true;
        }
        false
    }

    /// Asks inside the window ending at `now`.
    pub(crate) fn recent(&self, now: u64) -> usize {
        self.asks
            .iter()
            .filter(|at| now.saturating_sub(**at) < REGENERATION_WINDOW_SECS)
            .count()
    }
}

/// What the loop should do about the panes it can see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Decision {
    /// The board has every pane its layout declared.
    Whole,
    /// The layout never said how many panes this run should have.
    Undeclared,
    /// A pane has been asked for and has not arrived yet.
    Waiting,
    /// Ask Zellij for one pane. Missing panes are healed one at a time: each
    /// arrival is its own `PaneUpdate`, and the next decision is made from what
    /// that manifest actually shows rather than from a plan made before it.
    Regenerate { missing: usize },
    /// The board is short and this run has stopped trying.
    GaveUp { missing: usize },
}

/// Decide what to do about `census`, given what has already been tried.
pub(crate) fn decide(
    census: Census,
    outstanding: Option<Outstanding>,
    budget: &Budget,
    now: u64,
) -> Decision {
    let Some(missing) = census.missing() else {
        return Decision::Undeclared;
    };
    if missing == 0 {
        return Decision::Whole;
    }
    if outstanding.is_some_and(|ask| !ask.is_stale(now)) {
        return Decision::Waiting;
    }
    if budget.has_room(now) {
        Decision::Regenerate { missing }
    } else {
        Decision::GaveUp { missing }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn census(declared: Option<usize>, present: usize) -> Census {
        Census { declared, present }
    }

    fn asked_at(now: u64) -> Option<Outstanding> {
        Some(Outstanding {
            asked_at: now,
            focus_before: None,
        })
    }

    #[test]
    fn a_whole_board_decides_nothing() {
        let full = census(Some(4), 4);
        assert!(full.is_whole());
        assert_eq!(full.missing(), Some(0));
        assert_eq!(decide(full, None, &Budget::default(), 100), Decision::Whole);
    }

    #[test]
    fn an_operators_extra_pane_is_not_a_surplus_to_be_trimmed() {
        // Five panes where four were declared: somebody opened one. Lisa counts
        // itself whole and never closes a pane to make the number agree.
        let extra = census(Some(4), 5);
        assert!(extra.is_whole());
        assert_eq!(extra.missing(), Some(0));
        assert_eq!(
            decide(extra, None, &Budget::default(), 100),
            Decision::Whole
        );
    }

    #[test]
    fn the_reported_board_asks_for_the_panes_it_lost() {
        // screen-design-4, as measured: two children where four were declared.
        let short = census(Some(4), 2);
        assert_eq!(short.missing(), Some(2));
        assert!(!short.is_whole());
        assert_eq!(
            decide(short, None, &Budget::default(), 100),
            Decision::Regenerate { missing: 2 }
        );
    }

    #[test]
    fn a_layout_that_never_said_how_many_heals_nothing() {
        let unknown = census(None, 2);
        assert_eq!(unknown.missing(), None);
        assert!(unknown.is_whole());
        assert_eq!(
            decide(unknown, None, &Budget::default(), 100),
            Decision::Undeclared
        );
    }

    #[test]
    fn an_outstanding_ask_holds_the_next_one_off() {
        let short = census(Some(4), 3);
        let budget = Budget::default();
        assert_eq!(
            decide(short, asked_at(100), &budget, 100),
            Decision::Waiting
        );
        assert_eq!(
            decide(
                short,
                asked_at(100),
                &budget,
                100 + ADOPTION_TIMEOUT_SECS - 1
            ),
            Decision::Waiting
        );
        // A pane that never arrived stops blocking, and the ask that produced it
        // has already been charged.
        assert_eq!(
            decide(short, asked_at(100), &budget, 100 + ADOPTION_TIMEOUT_SECS),
            Decision::Regenerate { missing: 1 }
        );
    }

    #[test]
    fn a_pane_that_dies_on_arrival_stops_after_three_asks() {
        let short = census(Some(4), 3);
        let mut budget = Budget::default();
        for ask in 1..=MAX_REGENERATIONS {
            assert_eq!(
                decide(short, None, &budget, 100),
                Decision::Regenerate { missing: 1 },
                "ask {ask} should have been allowed"
            );
            let exhausted = budget.spend(100);
            assert_eq!(exhausted, ask == MAX_REGENERATIONS);
        }
        assert_eq!(
            decide(short, None, &budget, 100),
            Decision::GaveUp { missing: 1 }
        );
    }

    #[test]
    fn giving_up_is_sticky_for_the_run() {
        // A budget that re-arms once the window rolls past is the spin it exists
        // to prevent, at ten minutes a turn. Restarting the loop is the way back.
        let mut budget = Budget::default();
        for _ in 0..MAX_REGENERATIONS {
            budget.spend(100);
        }
        assert!(budget.has_given_up());
        let long_after = 100 + REGENERATION_WINDOW_SECS * 10;
        assert_eq!(budget.recent(long_after), 0);
        assert!(!budget.has_room(long_after));
        assert_eq!(
            decide(census(Some(4), 3), None, &budget, long_after),
            Decision::GaveUp { missing: 1 }
        );
    }

    #[test]
    fn the_window_slides_so_a_slow_trickle_of_losses_keeps_healing() {
        let mut budget = Budget::default();
        let mut now = 100;
        for loss in 0..10 {
            assert!(
                budget.has_room(now),
                "loss {loss} an hour apart should still be healable"
            );
            assert!(!budget.spend(now));
            now += REGENERATION_WINDOW_SECS + 1;
        }
        assert!(!budget.has_given_up());
    }

    #[test]
    fn a_stale_ask_is_stale_only_after_the_adoption_window() {
        let ask = Outstanding {
            asked_at: 100,
            focus_before: Some(7),
        };
        assert!(!ask.is_stale(100));
        assert!(!ask.is_stale(100 + ADOPTION_TIMEOUT_SECS - 1));
        assert!(ask.is_stale(100 + ADOPTION_TIMEOUT_SECS));
    }
}
