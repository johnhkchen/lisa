# Review — T-052-02-02 fold-the-echoes

A fact that repeats now costs one line and an honest multiplier. The 100-entry
activity ring holds 100 distinct facts regardless of how many echoes each
absorbed.

## What changed

Two files, three commits, no files created or deleted.

| Commit | Message | Files |
|---|---|---|
| `2fe36ac` | fix(plugin): fold consecutive identical activity events at append | `lib.rs` |
| `d081e72` | feat(plugin): render folded activity echoes as a trailing multiplier | `lib.rs`, `ui.rs` |
| `9e4be61` | fix(plugin): carry the fold multiplier into the state dump | `lib.rs` |

### `crates/lisa-plugin/src/lib.rs`

- **`LoggedActivity`** gained `count: u32`. `at`'s doc now says a fold overwrites
  it with the latest occurrence's instant.
- **`log_activity_at`** folds: an event equal to `activity_log.last()` bumps that
  entry's `count` and refreshes its `at` and returns, instead of pushing. The
  early return is what makes the cap unreachable from the fold path.
- **`activity_event_to_ui_entry`** copies `entry.count` alongside `entry.at`.
- **`format_snapshot`** (Shift+D dump) iterates the envelopes so it can apply the
  same multiplier the feed uses.
- **`activity_events()`** became `#[cfg(test)]` — see Concerns.

### `crates/lisa-plugin/src/ui.rs`

- **`ActivityEntry`** gained `pub count: u32`.
- **`with_repeat_tag(message, count)`** added, `pub(crate)`; `count <= 1` returns
  the message unchanged.
- **Both renderers** rebind `message` through it immediately before their
  existing `format!`. No `match` arm was touched.
- **`render_activity_log`** widened to `pub(crate)`, matching `render_threads`.

## Why equality and not "renders the same"

The ticket asked to fold "when the incoming event renders identical to the newest
entry". The implementation folds on structural `ActivityEvent` equality, which is
strictly stronger: equal events necessarily render identically, so every fold
performed is one the literal reading would also have performed.

The refinement is deliberate and is the one judgment call in this ticket worth a
reviewer's attention. The ring backs the Shift+D audit dump as well as the feed,
and several variants carry fields the feed drops (`pane_id`,
`ArtifactCreated.phase`, `SessionTimedOut.elapsed_secs`, shown only in whole
minutes). Folding on rendered sameness would collapse two genuinely different
facts into one dump line — two `SessionLaunch` events on different panes, two
timeouts ten seconds apart both reading "60m". Worse, every demoted variant
projects to `None`, so a `DagRecomputed` following a `TicketPhaseChanged` "renders
identically" (both render nothing) and would fold two unrelated facts.

That contradicts the stance T-052-02-01 set and this codebase states in
`DeclineReason`'s doc: demotion, not erasure. The rationale is written into
`log_activity_at`'s doc comment so a future loosening has to argue with it.

**The cost:** two events that render alike but differ underneath print two lines
rather than one `(x2)`. That is the correct failure direction — two lines that
are honestly two facts, versus one line that silently ate one — and it costs
nothing on the ticket's motivating cases, which are literal byte-identical
echoes.

## Acceptance criteria

| # | Criterion | Evidence |
|---|---|---|
| 1 | Three identical events → one `(x3)` line with latest timestamp; an intervening event breaks the fold | `three_identical_events_fold_into_one_counted_line` (len 1, count 3, `at == now+90`, and `assert_ne!` against the first instant), `folded_line_renders_one_entry_with_the_multiplier` (end-to-end through `to_ui_state()`), `an_intervening_event_breaks_the_fold` |
| 2 | Ring counts a folded line once; 100 distinct facts fit at the cap | `distinct_facts_fill_the_ring_regardless_of_echoes` — 100 facts × 2–4 echoes each → `len() == 100`, oldest fact still present, per-entry counts correct; then one more fact evicts `fact-0`, not an echo |
| 3 | Fold in `log_activity`, not the renderers; projection stays a pure map | Fold lives in `log_activity_at`, the workspace's sole mutable seam on `activity_log`. `projection_preserves_the_count` asserts behaviourally that projecting twice yields identical counts |
| 4 | Near-identical events never fold | `near_identical_events_never_fold` — table over different ticket, different phase, different message, and same message at different severity |
| 5 | `just check` green | **Exit 0**, judged by exit code |

## Test coverage

Ten tests added: six in `lib.rs`, four in `ui.rs`. All unit tests in existing
modules; no integration harness, because the feature lives between one append
function and two pure renderers. Clock assertions go through the established
`feed_test_instant` / `FEED_TEST_NOW_SECS` fixture — no sleeps.

**The tests were verified to bite.** Three deliberate regressions were introduced
and reverted:

| Mutation | Tests that failed |
|---|---|
| Fold arm disabled | 5 |
| Stamp refresh removed | 2 |
| Multiplier moved to the front of the line | 4 |

The stamp-refresh mutation is the instructive one: only two tests caught it. A
fold that kept the *oldest* stamp — silently reintroducing the stale-age bug
T-052-01-01 fixed — would have passed everything except the explicit `assert_ne!`
against the first occurrence's instant. That assertion is load-bearing; a future
edit that removes it as redundant would open the hole back up.

**Regression evidence from the existing suite:** all 464 pre-existing tests passed
unchanged at step 1, and the seven `ActivityEntry` fixtures in `ui.rs` pass with
`count: 1` and their assertions untouched — that is the proof that an unfolded
line renders byte-identically to before.

## Open concerns

1. **`activity_events()` is now `#[cfg(test)]`.** The dump was its last
   production caller. This surfaced as a `-D dead-code` failure in `just check`,
   not as a warning, and it is an accurate description of what the method became
   — the assertion vocabulary of ~40 tests — rather than a silenced lint. The
   alternative, leaving the dump on bare events, was rejected because it would
   erase multiplicity from the audit surface. If a future production reader needs
   bare events again, drop the attribute; nothing else has to change.

2. **`render_activity_log` widened to `pub(crate)`.** Needed so `lib.rs` tests
   can assert the fold end-to-end from a `State`. Precedent exists
   (`render_threads`), but it is a visibility widening driven by a test, and a
   reviewer may reasonably prefer the weaker projection-level assertion instead.

3. **Deliberate non-goals**, all decided in Design and none of them gaps:
   - No folding across a gap — only the newest entry is a candidate, so the feed
     stays a chronology.
   - No time window on folds. A fold has one reason to fail, not two.
   - Events that render alike but differ underneath do not fold (see above).
   - No cap on `count` beyond `u32` saturation.

4. **Pre-existing flake, not introduced here and not fixed here.**
   `lisa-cli`'s `triage_agent::tests::bounded_runner_returns_valid_proposal_and_surfaces_failure`
   failed once with `TimedOut` during a full-load `just check`. It gives a
   `printf` shell script a 2-second wall-clock deadline and exceeded it while 380
   tests ran in parallel right after a full rebuild. It passes in isolation and
   on a clean re-run, and this ticket touches only `crates/lisa-plugin`. Worth a
   ticket of its own — a 2s deadline on a spawned process is load-sensitive by
   construction — but fixing it here would mean editing a file this ticket does
   not own.

## Nothing left staged

`git status --short crates/` is empty. Every ticket-owned change went through
`lisa commit-ticket` with exact `--include` paths; no ordinary `git add` or
`git commit` was used for ticket work.
