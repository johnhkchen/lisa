//! Which member of the agent stack is expanded while nobody is looking at it.
//!
//! `lisa loop` lays the agent panes out as a zellij *stack*: one member is
//! expanded and the rest are one-line title bars. Zellij expands whichever
//! member was focused last, so an operator who walks down to the plugin pane
//! leaves the stack showing whatever they passed through on the way — with
//! `max_threads = 2` that is usually a spare that has never held a ticket, and
//! it occupies 70% of the tab while the two working sessions are collapsed.
//!
//! This module decides, from facts the plugin already holds, which member
//! *should* be expanded. It is deliberately pure: the caller supplies the
//! observation (a [`StackView`] read off `PaneUpdate`) and the candidates, and
//! gets back either a pane to expand or `None`. Every rule that says "do
//! nothing" lives here, where it can be tested without a terminal.
//!
//! The two constraints, in the order they decide the design:
//!
//! 1. **Focus is never moved.** If the operator's focus is inside the stack,
//!    lisa does nothing at all — no promotion, no reordering. The stack is
//!    theirs while they are in it. The promotion itself is focus-free: see
//!    [`expands_without_focus`] for the one zellij call that separates the two,
//!    and for what happens on the versions that cannot.
//! 2. **Nothing static.** The generated layout carries no `expanded` attribute
//!    (that was tried; it re-applied on every terminal resize and snapped focus
//!    to the first pane). This runs at runtime, in response to state.

use std::collections::HashMap;

use lisa_core::types::AttemptLease;

/// What the last `PaneUpdate` said about the agent stack.
///
/// Both facts are read off pane geometry and focus flags rather than any
/// dedicated API: zellij's `PaneInfo` has no "is stacked"/"is expanded" field,
/// but a collapsed stack member is exactly one row tall (its title bar) while
/// the expanded one holds the rest of the stack's height.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct StackView {
    /// The agent pane currently expanded, if the panes look like a stack at
    /// all: exactly one agent pane taller than its title bar, and at least one
    /// collapsed beside it. `None` means "this is not a stack right now" —
    /// a single agent pane, a broken-out layout, or a manifest that arrived
    /// mid-relayout — and nothing is promoted into a shape we do not recognise.
    pub(crate) expanded: Option<u32>,
    /// Whether any client's focus is inside the stack. Any client counts: the
    /// conservative reading of a shared session is that somebody is reading.
    pub(crate) focus_inside: bool,
}

/// The geometry and focus of one agent pane, as `PaneUpdate` reports it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PaneObservation {
    pub(crate) pane_id: u32,
    /// Total rows including the frame. A collapsed stack member is 1.
    pub(crate) rows: usize,
    pub(crate) focused: bool,
}

/// Read the stack's state off this tick's agent-pane observations.
pub(crate) fn observe(panes: &[PaneObservation]) -> StackView {
    let focus_inside = panes.iter().any(|p| p.focused);
    let mut open = panes.iter().filter(|p| p.rows > 1);
    let expanded = match (open.next(), open.next()) {
        // Exactly one open pane beside at least one collapsed one: a stack.
        (Some(only), None) if panes.iter().any(|p| p.rows <= 1) => Some(only.pane_id),
        _ => None,
    };
    StackView {
        expanded,
        focus_inside,
    }
}

/// The heartbeat that promoted a pane, remembered per pane.
///
/// The lease is carried beside the ordinal so a recycled pane cannot inherit
/// its predecessor's activity: a heartbeat only counts while the seat still
/// holds the exact attempt that produced it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HeartbeatMark {
    pub(crate) lease: AttemptLease,
    /// Monotonic arrival ordinal. A counter rather than a clock: the question
    /// is only which heartbeat was admitted last, and a counter cannot be
    /// perturbed by the host clock moving.
    pub(crate) seq: u64,
}

/// A seat's promotion candidacy, as the scheduler holds it.
#[derive(Debug, Clone)]
pub(crate) struct Candidate {
    pub(crate) pane_id: u32,
    /// The attempt this seat is currently holding, or `None` for an idle seat.
    /// A pane with no lease is not a candidate at all, however recently
    /// anything touched it.
    pub(crate) lease: Option<AttemptLease>,
}

/// The pane that should be expanded, or `None` when nothing should move.
///
/// `None` covers every "do nothing" case and they are all deliberate:
/// the operator is inside the stack; the panes are not a stack; no seat holds
/// a lease; no leased seat has produced a heartbeat yet (between runs, or
/// before the first assignment, nothing has been shown to be working); or the
/// pane that should be expanded already is.
pub(crate) fn promotion_target(
    view: StackView,
    candidates: &[Candidate],
    heartbeats: &HashMap<u32, HeartbeatMark>,
) -> Option<u32> {
    if view.focus_inside {
        return None;
    }
    let expanded = view.expanded?;
    let newest = candidates
        .iter()
        .filter_map(|candidate| {
            let lease = candidate.lease.as_ref()?;
            let mark = heartbeats.get(&candidate.pane_id)?;
            // A heartbeat belongs to the attempt that produced it. A pane
            // re-leased for the next ticket starts from silence.
            (&mark.lease == lease).then_some((mark.seq, candidate.pane_id))
        })
        .max()?
        .1;
    (newest != expanded).then_some(newest)
}

/// Whether this zellij can expand a stack member *without* focusing it.
///
/// The plugin asks for the expansion with `show_pane_with_id(pane, false)` —
/// the `false` is `should_float_if_hidden`, the only other argument the 0.43
/// SDK this plugin is built against takes. Whether the request costs the
/// operator their focus is decided entirely by the zellij *server* on the other
/// end of it, and the two versions decide it differently:
///
/// - **0.44 and later** added a third `should_focus_pane` field to the
///   `ShowPaneWithId` payload. A 0.43 client never writes it, so it arrives as
///   protobuf's default `false` and the command routes to
///   `UnsuppressOrExpandPane` → `Tab::unsuppress_or_expand_pane`, which is
///   documented "does not focus it" and, for a pane that is not suppressed,
///   expands it inside its stack. Focus is untouched.
/// - **0.43** has no such field and routes the same command unconditionally to
///   `FocusPaneWithId`: it expands the member *by focusing it*, which is
///   exactly the behaviour this ticket forbids.
///
/// So the promotion is version-gated rather than attempted-and-hoped: on 0.43
/// lisa leaves the stack alone and says so once, because the only expansion
/// available there moves the operator's focus.
pub(crate) fn expands_without_focus(zellij_version: &str) -> bool {
    let Some((major, minor)) = parse_version(zellij_version) else {
        // An unreadable version is not a licence to move somebody's focus.
        return false;
    };
    (major, minor) >= (0, 44)
}

/// Parse the leading `major.minor` out of a zellij version string, which
/// arrives as `0.44.3` and has historically also been seen as `zellij 0.44.3`
/// and `v0.44.3`.
fn parse_version(raw: &str) -> Option<(u32, u32)> {
    let token = raw
        .split_whitespace()
        .last()?
        .trim_start_matches(['v', 'V'])
        .trim();
    let mut parts = token.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    Some((major, minor))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lease(ticket: &str, attempt: u64) -> AttemptLease {
        AttemptLease {
            ticket_id: ticket.to_string(),
            attempt_id: attempt,
        }
    }

    fn pane(pane_id: u32, rows: usize, focused: bool) -> PaneObservation {
        PaneObservation {
            pane_id,
            rows,
            focused,
        }
    }

    fn working(pane_id: u32, ticket: &str) -> Candidate {
        Candidate {
            pane_id,
            lease: Some(lease(ticket, 1)),
        }
    }

    fn idle(pane_id: u32) -> Candidate {
        Candidate {
            pane_id,
            lease: None,
        }
    }

    fn beats(marks: &[(u32, AttemptLease, u64)]) -> HashMap<u32, HeartbeatMark> {
        marks
            .iter()
            .map(|(pane_id, lease, seq)| {
                (
                    *pane_id,
                    HeartbeatMark {
                        lease: lease.clone(),
                        seq: *seq,
                    },
                )
            })
            .collect()
    }

    #[test]
    fn a_stack_is_one_open_pane_among_collapsed_ones() {
        let view = observe(&[
            pane(0, 1, false),
            pane(1, 1, false),
            pane(2, 22, false),
            pane(3, 1, false),
        ]);
        assert_eq!(view.expanded, Some(2));
        assert!(!view.focus_inside);
    }

    #[test]
    fn focus_inside_the_stack_is_seen_on_any_member() {
        let view = observe(&[pane(0, 1, false), pane(1, 22, true), pane(2, 1, false)]);
        assert!(view.focus_inside);
    }

    #[test]
    fn a_focused_but_collapsed_member_still_counts_as_focus_inside() {
        // Zellij will happily leave focus on a one-line member (it is what an
        // unguarded expansion does to the pane somebody is reading), so focus
        // is read from the focus flag and never inferred from the geometry.
        let view = observe(&[pane(0, 1, true), pane(1, 22, false), pane(2, 1, false)]);
        assert!(view.focus_inside);
        assert_eq!(view.expanded, Some(1));
    }

    #[test]
    fn panes_that_are_not_a_stack_are_not_recognised() {
        // Two open panes: the operator broke the stack apart, or the manifest
        // caught a relayout mid-flight. Either way, nothing to promote into.
        assert_eq!(
            observe(&[pane(0, 12, false), pane(1, 12, false)]).expanded,
            None
        );
        // A lone pane is not a stack either.
        assert_eq!(observe(&[pane(0, 22, false)]).expanded, None);
    }

    #[test]
    fn the_newest_heartbeat_holding_a_lease_is_promoted() {
        let view = StackView {
            expanded: Some(3),
            focus_inside: false,
        };
        let candidates = [working(0, "T-1"), working(1, "T-2"), idle(2), idle(3)];
        let marks = beats(&[(0, lease("T-1", 1), 7), (1, lease("T-2", 1), 9)]);
        assert_eq!(promotion_target(view, &candidates, &marks), Some(1));
    }

    #[test]
    fn focus_inside_the_stack_promotes_nothing() {
        let view = StackView {
            expanded: Some(3),
            focus_inside: true,
        };
        let candidates = [working(0, "T-1")];
        let marks = beats(&[(0, lease("T-1", 1), 7)]);
        assert_eq!(promotion_target(view, &candidates, &marks), None);
    }

    #[test]
    fn an_idle_pane_never_displaces_a_working_one() {
        let view = StackView {
            expanded: Some(0),
            focus_inside: false,
        };
        // Pane 3 is idle and its old heartbeat is the newest thing on record;
        // it holds no lease, so it is not a candidate.
        let candidates = [working(0, "T-1"), idle(3)];
        let marks = beats(&[(0, lease("T-1", 1), 4), (3, lease("T-0", 1), 99)]);
        assert_eq!(promotion_target(view, &candidates, &marks), None);
    }

    #[test]
    fn a_recycled_pane_does_not_inherit_its_predecessors_heartbeat() {
        let view = StackView {
            expanded: Some(0),
            focus_inside: false,
        };
        // Pane 1 has just been leased for T-3; the heartbeat on record belongs
        // to the attempt that vacated it, so pane 0 stays expanded.
        let candidates = [working(0, "T-1"), working(1, "T-3")];
        let marks = beats(&[(0, lease("T-1", 1), 4), (1, lease("T-2", 1), 99)]);
        assert_eq!(promotion_target(view, &candidates, &marks), None);
        // Pane 0 is already the expanded one, so there is nothing to move.
        let elsewhere = StackView {
            expanded: Some(2),
            focus_inside: false,
        };
        assert_eq!(promotion_target(elsewhere, &candidates, &marks), Some(0));
    }

    #[test]
    fn every_seat_idle_moves_nothing() {
        let view = StackView {
            expanded: Some(3),
            focus_inside: false,
        };
        let candidates = [idle(0), idle(1), idle(2), idle(3)];
        assert_eq!(promotion_target(view, &candidates, &HashMap::new()), None);
    }

    #[test]
    fn a_leased_seat_that_has_not_worked_yet_moves_nothing() {
        // Between the assignment and the first tool call there is no evidence
        // of work, and promoting on assignment alone is the smaller design the
        // operator declined.
        let view = StackView {
            expanded: Some(3),
            focus_inside: false,
        };
        let candidates = [working(0, "T-1")];
        assert_eq!(promotion_target(view, &candidates, &HashMap::new()), None);
    }

    #[test]
    fn the_pane_that_should_be_expanded_already_is() {
        let view = StackView {
            expanded: Some(0),
            focus_inside: false,
        };
        let candidates = [working(0, "T-1")];
        let marks = beats(&[(0, lease("T-1", 1), 7)]);
        assert_eq!(promotion_target(view, &candidates, &marks), None);
    }

    #[test]
    fn an_unrecognised_shape_promotes_nothing() {
        let view = StackView {
            expanded: None,
            focus_inside: false,
        };
        let candidates = [working(0, "T-1")];
        let marks = beats(&[(0, lease("T-1", 1), 7)]);
        assert_eq!(promotion_target(view, &candidates, &marks), None);
    }

    #[test]
    fn only_zellij_that_can_expand_without_focusing_is_used() {
        assert!(expands_without_focus("0.44.3"));
        assert!(expands_without_focus("0.45.0"));
        assert!(expands_without_focus("1.0.0"));
        assert!(expands_without_focus("zellij 0.44.0"));
        assert!(expands_without_focus("v0.44.1"));
        // 0.43 expands only by focusing, which this ticket forbids.
        assert!(!expands_without_focus("0.43.1"));
        assert!(!expands_without_focus("0.40.1"));
        // An unreadable version is treated as one that cannot.
        assert!(!expands_without_focus(""));
        assert!(!expands_without_focus("unknown"));
        assert!(!expands_without_focus("0"));
    }
}
