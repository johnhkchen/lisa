# Design — T-052-02-02 fold-the-echoes

## The decision in one line

Fold in `log_activity_at` when the incoming `ActivityEvent` is **structurally
equal** to the newest entry's event: bump a `count` on that entry and overwrite
its `at`, instead of pushing. Carry `count` through the envelope into
`ui::ActivityEntry` and let both renderers hang a trailing `(xN)` on the line.

## Where the fold happens

Research §2 established that `log_activity_at` is the sole mutable seam: one
`push`, one `remove(0)`, no other writer in the workspace. Placing the fold there
satisfies AC3 by construction rather than by discipline — there is no caller that
can append without passing through it, so no future emitter can accidentally opt
out of folding.

The alternative — folding in the projection at `lib.rs:9031` — is rejected
outright by the ticket and by arithmetic. The projection runs every render, on a
five-second cadence; a dedup pass there is an O(n) rescan of 100 entries every
five seconds forever, to recompute an answer that was already knowable at append
time, when the comparison is O(1) against one neighbour. It would also leave the
ring itself full of echoes, so `MAX_ACTIVITY_LOG` would still mean "100 lines"
rather than "100 facts" and AC2 would fail. Append-time is the only place that
fixes both the display and the accounting.

## The fold predicate: structural equality, not rendered equality

The ticket phrases the trigger as "when the incoming event *renders* identical to
the newest entry". Research §5 showed these are different relations: several
variants carry fields the feed drops (`pane_id`, `ArtifactCreated.phase`,
`SessionTimedOut.elapsed_secs` shown only in whole minutes), and the demoted
variants render nothing at all. So the two candidate predicates:

**Option A — structural equality (`newest.event == incoming`).** Chosen.

**Option B — rendered-line equality.** Project both entries through
`activity_event_to_ui_entry` and compare the resulting feed lines; fold when they
match.

Option B is rejected on the codebase's own stated principle. The ring is not only
the feed's backing store — it is also the Shift+D audit dump (Research §3), and
`DeclineReason`'s doc comment and the T-052-02-01 invariant both spell out the
stance: *demotion, not erasure*; the question still has an answer, it just lives
in the dump. Folding on rendered equality collapses entries that differ in
substance:

- Two `SessionLaunch` events for the same ticket and command on **different
  panes** would fold, and the second `pane_id` would vanish from the audit record.
- Two `SessionTimedOut` at 3610s and 3620s both render "60m" and would fold,
  losing the distinct elapsed measurements.
- Worst: every demoted variant projects to `None`. Under a naive "renders
  identical" reading, a `DagRecomputed{5}` following a `TicketPhaseChanged{...}`
  renders identically (both render nothing) and would fold two unrelated facts
  into one dump line. Option B would need a special case saying non-projecting
  events never fold — at which point the predicate is no longer "renders
  identical" anyway, and the non-projecting variants keep flooding the ring.

Option A avoids all of this because equality is *strictly stronger* than rendered
identity: **equal events necessarily render identically**, so every fold A
performs is one B would also have performed, and every entry A folds is
indistinguishable from its predecessor in the feed, in the dump, and in
`activity_events()` — nothing anywhere can tell the difference except the count,
which is exactly the fact being added. It is a sound refinement of what the
ticket asked for, deliberately conservative in the one direction where being
wrong destroys evidence.

The cost of the refinement is a missed fold when two events render alike but
differ underneath — two timeouts ten seconds apart print two "60m" lines rather
than one "(x2)". That is the correct failure direction: two lines that are
honestly two facts, versus one line that silently ate one. And it costs nothing
against the ticket's motivating cases, which are literal echoes: a `Warning`
re-fired on consecutive polls and a retried sweep re-emit byte-identical events,
so they carry identical `String` payloads and fold under A.

`ActivityEvent` already derives `PartialEq, Eq` (`types.rs:975`), so A needs no
new derive; B would need `PartialEq` added to `ui::ActivityType` or a string
comparison per append.

## Window of one, not many

Only the **newest** entry is a fold candidate. A different event arriving between
two identical ones breaks the fold and the second identical one starts a fresh
entry — this is AC1's second half, and it is also what keeps the fold honest:
the feed stays a chronology. A wider lookback (fold against any of the last k
entries) would let a line's position in the feed disagree with its timestamp, and
would make "A B A B A B" collapse into two entries that each claim to have
happened at an instant they interleaved around. Rejected.

Consequence worth naming: the phase-transition pair from
`log_phase_transition` (`PhaseCompleted` then `TicketPhaseChanged`) never folds
with itself, and a repeat of that pair does not fold either, because the two
events alternate. That is correct — `log_phase_transition` already has its own
idempotence guard (`logged_transitions`, T-052-02-01), and the fold is not a
substitute for it.

## Carrying the count outward

`count: u32` on `LoggedActivity`, initialised to 1 and incremented on fold.
`u32` over `usize` for a stable WASM/native footprint; over `NonZeroU32` because
the invariant is trivially local (constructed at 1, only ever incremented) and
the ergonomic tax of `NonZeroU32` arithmetic buys nothing here.

`ui::ActivityEntry` grows a matching `pub count: u32`. This keeps the projection
a pure per-entry map — `activity_event_to_ui_entry` copies `entry.count` beside
`entry.at`, no scanning, no state — so AC3/N4 holds literally: the projection is
still `filter_map` over stored entries, and every field it produces comes from
the entry it was handed.

The field is added rather than derived at render time on purpose. A renderer that
recomputed multiplicity would be the O(n) rescan the ticket forbids, and would
have to duplicate the logic in both renderers.

## Rendering the tag

Both renderers build a `message` string in a per-variant `match`, truncate free
text (40 chars in the full feed, 50 in alerts), then emit one `format!` line.
The tag is appended **after** truncation, at the single emission point in each
renderer, via one shared helper:

```
fn with_repeat_tag(message: String, count: u32) -> String   // "sweep retried (x3)"
```

Appending after truncation is deliberate: the multiplier is the part of the line
the operator cannot reconstruct, so it must never be the part that the `...`
eats. `count == 1` returns the message untouched, so every existing rendered line
is byte-identical to today's — the change is invisible until something actually
echoes, which keeps the existing renderer fixtures honest rather than rewritten.

One helper called from two sites, rather than folded into each `match` arm: the
arms are already duplicated across the two renderers with divergent truncation
widths (Research §4), and adding fourteen more copies of the same suffix logic is
how that divergence got there.

## The dump gets the multiplier too

The Shift+D dump reads `activity_events()`, which drops the envelope and yields
bare `&ActivityEvent`. Left alone, a folded triple would appear in the audit
record as a *single* occurrence — the fold would have erased "this happened three
times" from the one surface whose whole job is to still have the answer. That
contradicts the demote-never-erase stance this design leans on to justify
Option A, so the dump must carry the count as well: iterate the envelopes and
suffix the same `(xN)` when `count > 1`.

`activity_events()` stays as-is for the ~40 tests that assert over bare events.

## Ring accounting

Nothing about eviction changes. Folding simply does not push, so the length check
and `remove(0)` are untouched and a folded line occupies exactly one slot no
matter how many echoes it absorbed — AC2 falls out of the placement rather than
needing its own mechanism. `MAX_ACTIVITY_LOG = 100` now means 100 distinct facts.

Eviction stays `remove(0)` on a `Vec`; folding strictly *reduces* how often that
memmove runs, so there is no reason to touch it here.

## What this does not do

- No fold across a gap; no time-window ("fold only if within 60s"). The ticket
  asks for adjacency, and a time window would add a second reason a fold can fail
  without adding a fact the operator wanted.
- No cap on `count`. A `u32` saturating at 4 billion echoes is not a scenario.
- No change to which events project. T-052-02-01 owns that list and its
  invariant comment; this ticket adds a multiplier to lines, not new lines.
