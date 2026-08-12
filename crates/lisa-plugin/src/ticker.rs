//! The plugin pane's title, composed as a ticker.
//!
//! The title bar above the dashboard is the widest constant strip of text on a
//! `lisa loop` screen, and until now it read
//! `file:/var/folders/…/lisa-plugin-8fd16992c4a2b9fd.wasm` — a temp path and a
//! content hash, identical on every project. This module turns that row into a
//! line that says what the loop is doing and changes as that changes.
//!
//! # What it carries, and why only this
//!
//! Four facts, in this order, joined by ` · `:
//!
//! 1. **The project.** The same row appears on every station; on a desk running
//!    three loops it is the only thing that tells them apart.
//! 2. **The state** — `2/4 working`, `idle`, `all done`, prefixed by `paused`
//!    when scheduling is held. One glance answers "is this loop alive".
//! 3. **What just finished**, for [`DONE_WINDOW_SECS`] — so a completion is
//!    visible to somebody who was not looking at the moment it happened.
//! 4. **The attempt running longest**, with its elapsed time and a `+N` for the
//!    rest. This answers the question a glance actually asks, which is not
//!    "what is running" but "has it been running too long".
//!
//! When nothing is working, the tail carries `4/9 done` instead: a stopped loop
//! should still say how far the board got.
//!
//! Everything else the plugin knows was left out on purpose. The dashboard three
//! rows below already shows slots, threads, alerts, the desk and the DAG; a
//! title that repeats them is a second copy that can disagree with the first.
//! The title's one advantage is being readable when the dashboard is scrolled,
//! in another view, or on a station across the room, so it carries only what
//! survives that distance.
//!
//! # Not a second computation
//!
//! [`compose`] is a pure function of the [`PluginState`] the dashboard is about
//! to render, plus the last `TicketPhaseChanged → Done` from the same activity
//! ring the feed is built from. It reads; it does not recompute. `2/4 working`
//! counts the same `active_threads` and `slots` the status line counts, and
//! `4/9 done` counts the same `tickets`.
//!
//! # Not a flicker
//!
//! Two rules keep the row still enough to read:
//!
//! - **Elapsed time is whole minutes.** The threads table renders `2m 30s`,
//!   which changes every second; a title that did the same would be rewritten
//!   on every render. Minutes give the row its heartbeat — it moves once a
//!   minute when nothing else happens — without ever moving faster than a
//!   glance.
//! - **Ties break on ticket id.** `active_threads` arrives from a `HashMap`, so
//!   two attempts started in the same second have no stable order. Picking the
//!   longest-running by `(started_at, ticket_id)` means the same facts always
//!   compose the same string, instead of alternating between two names at
//!   render speed.
//!
//! The caller applies the string only when it differs from the last one it
//! applied, so a still loop makes no host calls at all.

use std::path::Path;
use std::time::Duration;

use crate::ui::{PluginState, TicketStatus};

/// How long a finished ticket keeps its place in the title.
///
/// Long enough that somebody who looked away during a completion still sees it,
/// short enough that the row is never bragging about old news.
pub(crate) const DONE_WINDOW_SECS: u64 = 90;

/// Longest title lisa hands zellij, in Unicode scalar values.
///
/// The terminal truncates long before this; the bound exists so a pathological
/// ticket id cannot make the title unbounded. Matches the agent panes' own
/// `MAX_PANE_NAME_CHARS` so the two rows are cut by the same rule.
pub(crate) const MAX_TICKER_CHARS: usize = 80;

/// Longest project label, in Unicode scalar values. The project is first, so it
/// is the one segment that always survives truncation — it must not be able to
/// consume the whole row on its own.
const MAX_PROJECT_CHARS: usize = 24;

/// Shown when the project directory has no usable name (or, in native tests,
/// when there is no host cwd at all).
const FALLBACK_PROJECT: &str = "lisa";

/// Everything the title is composed from.
pub(crate) struct Ticker<'a> {
    /// Project label, from [`project_label`].
    pub(crate) project: &'a str,
    /// The state the dashboard is about to render.
    pub(crate) state: &'a PluginState,
    /// The most recent ticket to reach Done and the instant it did, read from
    /// the plugin's activity ring. `None` when nothing has finished this run.
    pub(crate) last_done: Option<(&'a str, Duration)>,
}

/// Compose the plugin pane's title.
pub(crate) fn compose(ticker: Ticker<'_>) -> String {
    let state = ticker.state;
    let seats = state.slots.len();
    let working = state.active_threads.len();
    let total = state.tickets.len();
    let done = state
        .tickets
        .iter()
        .filter(|ticket| ticket.status == TicketStatus::Done)
        .count();

    // Ordered by what should survive truncation: the terminal cuts from the
    // right, so position is priority. Project first because it is what tells
    // three stations apart; the board's progress last because it is the only
    // segment still true a minute from now.
    let mut segments: Vec<String> = Vec::with_capacity(5);
    segments.push(ticker.project.to_string());

    // A paused loop with no threads left looks exactly like an idle one, and
    // the difference is whether it will ever start again on its own. It goes
    // ahead of the state it qualifies.
    if state.paused {
        segments.push("paused".to_string());
    }

    if working > 0 {
        // `seats.max(working)` rather than `seats`: before the first PaneUpdate
        // no slots are discovered yet, and `1/0 working` is worse than `1/1`.
        segments.push(format!("{}/{} working", working, seats.max(working)));
    } else if total == 0 {
        segments.push("idle".to_string());
    } else if done == total {
        segments.push("all done".to_string());
    } else {
        segments.push("idle".to_string());
    }

    // What just finished wins the place ahead of what is running, because it is
    // the fact that expires. The running attempt will still be there in a
    // minute; this will not.
    if let Some((ticket_id, at)) = ticker.last_done {
        if state.current_time.saturating_sub(at).as_secs() <= DONE_WINDOW_SECS {
            segments.push(format!("done {}", ticket_id));
        }
    }

    if let Some(longest) = state.active_threads.iter().min_by(|a, b| {
        a.started_at
            .cmp(&b.started_at)
            .then_with(|| a.ticket_id.cmp(&b.ticket_id))
    }) {
        let elapsed = elapsed_short(state.current_time.saturating_sub(longest.started_at));
        let others = working.saturating_sub(1);
        let more = if others > 0 {
            format!(" +{}", others)
        } else {
            String::new()
        };
        segments.push(format!("{} {}{}", longest.ticket_id, elapsed, more));
    } else if total == 0 {
        // Idle is never blank, and "idle" alone reads like a loop that lost its
        // board. Say which kind of nothing this is.
        segments.push("no tickets".to_string());
    } else if done < total {
        segments.push(format!("{}/{} done", done, total));
    }

    truncate(segments.join(" · "), MAX_TICKER_CHARS)
}

/// The title for a loop that has drained its board.
///
/// The finished screen returns before the dashboard state is built, so this is
/// the one title composed without it. It is the same string [`compose`] emits
/// for an all-done board, and a test holds the two together.
pub(crate) fn finished(project: &str) -> String {
    truncate(format!("{} · all done", project), MAX_TICKER_CHARS)
}

/// The project's label, from the loop's working directory.
///
/// Not the zellij session name: that one is bent to fit zellij's rules and may
/// carry a `-2` suffix from a collision. This is the directory as a person
/// says it.
pub(crate) fn project_label(root: &Path) -> String {
    let raw = root
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default();

    let cleaned = sanitize(&raw);
    if cleaned.is_empty() {
        return FALLBACK_PROJECT.to_string();
    }
    truncate(cleaned, MAX_PROJECT_CHARS)
}

/// Whole minutes, never seconds.
///
/// See the flicker note in the module docs: a per-second figure would rewrite
/// the title on every render for no gain a glance can use.
fn elapsed_short(elapsed: Duration) -> String {
    let minutes = elapsed.as_secs() / 60;
    if minutes == 0 {
        "<1m".to_string()
    } else if minutes < 60 {
        format!("{}m", minutes)
    } else {
        format!("{}h{:02}m", minutes / 60, minutes % 60)
    }
}

/// Drop control characters and collapse runs of whitespace.
///
/// A pane title is written straight into the terminal's title row; an escape
/// sequence arriving from a directory name has no business being executed
/// there.
fn sanitize(value: &str) -> String {
    let mut out = String::new();
    let mut pending_space = false;

    for ch in value.chars() {
        if ch.is_control() || ch.is_whitespace() {
            pending_space = !out.is_empty();
            continue;
        }
        if pending_space {
            out.push(' ');
            pending_space = false;
        }
        out.push(ch);
    }

    out
}

/// Bound a string by Unicode scalar values, so truncation never slices a
/// character in half.
fn truncate(value: String, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value;
    }
    let kept: String = value.chars().take(max_chars.saturating_sub(1)).collect();
    format!("{}…", kept)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::{ActiveThread, Phase, SlotInfo, TicketNode};

    const NOW: u64 = 1_800_000_000;

    fn ticket(id: &str, status: TicketStatus) -> TicketNode {
        TicketNode {
            id: id.to_string(),
            title: id.to_string(),
            phase: Phase::Implement,
            status,
            depends_on: Vec::new(),
        }
    }

    fn thread(id: &str, started_secs_ago: u64) -> ActiveThread {
        ActiveThread {
            ticket_id: id.to_string(),
            phase: Phase::Implement,
            started_at: Duration::from_secs(NOW - started_secs_ago),
            slot_number: 1,
            awaiting: false,
            route: None,
        }
    }

    /// A board of four tickets and four seats, nothing running.
    fn board() -> PluginState {
        PluginState {
            tickets: vec![
                ticket("T-062-01-01", TicketStatus::Done),
                ticket("T-062-01-02", TicketStatus::Done),
                ticket("T-062-01-03", TicketStatus::Ready),
                ticket("T-062-01-04", TicketStatus::Ready),
            ],
            slots: (1..=4)
                .map(|slot_number| SlotInfo {
                    ticket_id: None,
                    slot_number,
                    transitioning: false,
                })
                .collect(),
            current_time: Duration::from_secs(NOW),
            ..PluginState::default()
        }
    }

    fn compose_for(state: &PluginState, last_done: Option<(&str, Duration)>) -> String {
        compose(Ticker {
            project: "steer",
            state,
            last_done,
        })
    }

    // -- The three moments the ticket asks to see ---------------------------

    #[test]
    fn idle_says_so_and_says_how_far_the_board_got() {
        assert_eq!(
            compose_for(&board(), None),
            "steer · idle · 2/4 done",
            "a loop with nothing to do must not present as a blank row"
        );
    }

    #[test]
    fn one_seat_working_names_the_ticket_and_its_age() {
        let mut state = board();
        state.active_threads = vec![thread("T-062-01-03", 200)];
        state.slots[0].ticket_id = Some("T-062-01-03".to_string());

        assert_eq!(
            compose_for(&state, None),
            "steer · 1/4 working · T-062-01-03 3m"
        );
    }

    #[test]
    fn a_completion_is_still_on_the_row_a_minute_later() {
        let mut state = board();
        state.active_threads = vec![thread("T-062-01-03", 200)];
        let finished_at = Duration::from_secs(NOW - 60);

        assert_eq!(
            compose_for(&state, Some(("T-062-01-02", finished_at))),
            "steer · 1/4 working · done T-062-01-02 · T-062-01-03 3m",
            "somebody who looked away during the completion still sees it"
        );
    }

    #[test]
    fn a_stale_completion_gives_its_place_back() {
        let mut state = board();
        state.active_threads = vec![thread("T-062-01-03", 200)];
        let finished_at = Duration::from_secs(NOW - DONE_WINDOW_SECS - 1);

        assert_eq!(
            compose_for(&state, Some(("T-062-01-02", finished_at))),
            "steer · 1/4 working · T-062-01-03 3m"
        );
    }

    // -- Composition rules --------------------------------------------------

    #[test]
    fn several_seats_name_the_oldest_and_count_the_rest() {
        let mut state = board();
        state.active_threads = vec![
            thread("T-062-01-04", 90),
            thread("T-062-01-03", 3_900),
            thread("T-062-01-01", 120),
        ];

        assert_eq!(
            compose_for(&state, None),
            "steer · 3/4 working · T-062-01-03 1h05m +2",
            "the glance question is whether anything has been running too long"
        );
    }

    #[test]
    fn threads_started_in_the_same_second_do_not_alternate() {
        let mut state = board();
        state.active_threads = vec![thread("T-062-01-04", 200), thread("T-062-01-03", 200)];
        let first = compose_for(&state, None);

        state.active_threads.reverse();
        assert_eq!(
            first,
            compose_for(&state, None),
            "a tie broken by iteration order would flip the row at render speed"
        );
        assert!(first.contains("T-062-01-03 3m +1"));
    }

    #[test]
    fn a_fresh_attempt_reads_as_under_a_minute() {
        let mut state = board();
        state.active_threads = vec![thread("T-062-01-03", 12)];

        assert_eq!(
            compose_for(&state, None),
            "steer · 1/4 working · T-062-01-03 <1m"
        );
    }

    #[test]
    fn paused_is_said_before_the_state_it_qualifies() {
        let mut state = board();
        state.paused = true;

        assert_eq!(
            compose_for(&state, None),
            "steer · paused · idle · 2/4 done"
        );
    }

    #[test]
    fn a_drained_board_says_all_done_and_matches_the_finished_screen() {
        let mut state = board();
        for ticket in &mut state.tickets {
            ticket.status = TicketStatus::Done;
        }

        assert_eq!(compose_for(&state, None), "steer · all done");
        assert_eq!(compose_for(&state, None), finished("steer"));
    }

    #[test]
    fn an_empty_board_says_which_kind_of_nothing_it_is() {
        let mut state = board();
        state.tickets.clear();

        assert_eq!(compose_for(&state, None), "steer · idle · no tickets");
    }

    #[test]
    fn seats_are_never_reported_as_zero_while_work_runs() {
        let mut state = board();
        state.slots.clear();
        state.active_threads = vec![thread("T-062-01-03", 200)];

        assert!(compose_for(&state, None).contains("1/1 working"));
    }

    // -- Bounds -------------------------------------------------------------

    #[test]
    fn the_row_is_bounded_and_cut_on_a_character_boundary() {
        let mut state = board();
        state.active_threads = vec![thread(&"界".repeat(120), 200)];

        let title = compose_for(&state, None);
        assert_eq!(title.chars().count(), MAX_TICKER_CHARS);
        assert!(title.ends_with('…'));
        assert!(
            title.starts_with("steer · 1/4 working · "),
            "the project and the state are what survives a cut"
        );
    }

    #[test]
    fn forty_columns_still_carries_project_state_and_completion() {
        let mut state = board();
        state.active_threads = vec![thread("T-062-01-03", 200)];
        let title = compose_for(&state, Some(("T-062-01-02", Duration::from_secs(NOW))));

        let at_forty: String = title.chars().take(40).collect();
        assert_eq!(at_forty, "steer · 1/4 working · done T-062-01-02 ·");
    }

    // -- Project label ------------------------------------------------------

    #[test]
    fn the_project_is_the_directory_as_a_person_says_it() {
        assert_eq!(project_label(Path::new("/Users/j/swe/repos/lisa")), "lisa");
        assert_eq!(project_label(Path::new("/Users/j/swe/b28.dev")), "b28.dev");
        assert_eq!(
            project_label(Path::new("/Users/j/my project")),
            "my project"
        );
    }

    #[test]
    fn a_directory_with_no_usable_name_falls_back() {
        assert_eq!(project_label(Path::new("")), FALLBACK_PROJECT);
        assert_eq!(project_label(Path::new("/")), FALLBACK_PROJECT);
    }

    #[test]
    fn a_hostile_directory_name_cannot_write_escapes_into_the_title() {
        let label = project_label(Path::new("/tmp/a\u{1b}[2Jb"));
        assert_eq!(
            label, "a [2Jb",
            "the escape leaves as the separator it broke"
        );
        assert!(!label.chars().any(char::is_control));
    }

    #[test]
    fn a_long_directory_name_cannot_consume_the_row() {
        let label = project_label(&Path::new("/tmp").join("p".repeat(100)));
        assert_eq!(label.chars().count(), MAX_PROJECT_CHARS);
        assert!(label.ends_with('…'));
    }
}
