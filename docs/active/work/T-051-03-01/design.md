# Design — T-051-03-01 late-usage-join

## Problem restated

The terminal ledger row is written at completion with null tokens; the owning
capture lands afterward (rest-before-retire). Attribution must therefore be a
**late join**: a capture that appears in `.lisa/<client>/captures.jsonl` is
attributed to the ticket whose attempt reigned over that pane at capture time,
and the tokens are recorded as an **append-only correction** — never a mutation
of the published row. Unattributable captures quarantine by session id; a
quarantined capture drains if it later gains attribution evidence, and is
terminal-but-countable if it never does.

## Decision 1 — Join as a new append-only `usage-correction` record

**Chosen.** Add a `UsageCorrectionRecord` variant to `ProvenanceLedgerRecord`
with a required `record_type: "usage-correction"` discriminator (disjoint from
the others, placed before the fallback `Execution` variant). It carries the owner
`ticket_id` and its exact `attempt_lease`, the capture's `pane_id`, `session_id`,
`captured_at`, the provider `method`, the one-based `source_line` in
`captures.jsonl`, and the joined `tokens_in`/`tokens_out`, plus an `occurred_at`.

Rejected — **patch the original row in place.** Violates the append-only invariant
and the honest boundary ("no mutation of already-published ledger rows"); the
ledger has no update path and the doc guarantees row bytes are immutable.

Rejected — **a side file `.lisa/usage.jsonl`.** A second ledger fragments the
learning data, needs its own mixed-read discipline, and loses the free
schema-versioned reader the untagged enum already gives us. Corrections belong on
the one committable ledger.

**Idempotency key:** `(method, source_line)`. `captures.jsonl` is per-client,
append-only, and line-stable, so this uniquely identifies the source capture. The
sweep reads existing correction rows into a `HashSet<(String, u64)>` and never
writes a second correction for the same capture. No mutable cursor file — the same
"full rescan is safe" discipline the current code already relies on.

## Decision 2 — Attribution is a **pane reign**, not a closed window

**Chosen.** This is the crux. The winning capture's `captured_at` is strictly
*after* the owner's `ended_at`, so the existing inclusive `[started_at, ended_at]`
window (`owner_at`) can never cover it. Replace it with a reign model:

> On a pane, sort all occupants by `started_at`. Occupant R reigns from
> `R.started_at` until the next occupant's `started_at`. A capture at time `t`
> belongs to the occupant whose reign contains `t` — i.e. the occupant with the
> greatest `started_at ≤ t`.

This attributes A's post-`ended_at` rest capture to A (A is the last to have
started before it, and no successor has started yet), and a recycled pane's later
captures to the successor once it starts. Ambiguity (two *different* tickets tie
on the same max `started_at`) fails closed.

**The live-successor hazard and its fix.** A reign that is still open (no recorded
successor) must not swallow a capture that actually belongs to a *currently live
but not-yet-recorded* successor. The plugin knows live occupancy via
`self.threads` (`pane_id`, `started_at`, `ticket_id`). So the reign set is built
from **durable Execution rows (completed, attributable) + live threads (pending,
not attributable)**. Resolution returns one of three outcomes:

- `Attributed(&record)` — winning reign is a completed durable record → write a
  correction against `record.ticket_id` / `record.attempt_lease`.
- `Pending` — winning reign is a live thread → skip this poll; it converges to
  `Attributed` once that thread completes and its record lands. No correction, no
  quarantine.
- `Unowned` — no reign starts at or before `captured_at` (pane was free/unknown),
  or ambiguous tie → quarantine by session id.

Rejected — **extend the window to `ended_at + grace`.** A fixed grace is a guess;
too short drops the capture, too long steals a fast successor's capture. The
reign-until-next-start rule needs no magic constant and is exact given live-thread
awareness.

Rejected — **attribute by `session_id` (session-per-ticket).** Would be exact and
race-free, but the scheduler does not record which provider session an attempt
used; adding that is a larger cross-cutting change and the story explicitly frames
the key as the scheduler's pane×time history. Out of slice.

## Decision 3 — Retire inline attribution; the sweep is the single writer

**Chosen.** With reign attribution, the just-completed ticket's own capture is
joined by the same sweep that joins every other capture. Keeping the inline
`read_usage` stamp inside `emit_provenance_with_note` would leave two attribution
paths, make the row's tokens non-deterministic (present iff a capture happened to
race in early), and muddy the "row is null by construction" model. So:

- `emit_provenance_with_note` writes the terminal row with **null** tokens always.
- A new `sweep_usage_captures` (run each poll after `retire_resting_sessions`)
  owns all attribution: read captures → resolve reign → correction | quarantine |
  drain | skip.
- `read_usage` is removed; `quarantine_capture` stays (called by the sweep).

This changes the existing inline-attribution tests, which is expected: AC #1 says
tokens must arrive *via a correction record with the original row untouched*, so
inline stamping is now the wrong behavior. Legacy ledgers that already carry
non-null row tokens are honored by the corrected view (Decision 5) as a fallback.

Rejected — **keep inline stamping and add the sweep additively.** Lower test
churn, but two writers invite double-counting and a row whose nullness depends on
a race. The single-writer model is simpler to reason about and matches the story's
framing; the test churn is mechanical.

## Decision 4 — Quarantine drains by re-derivation, terminal rows persist

**Chosen.** The sweep treats `captures.jsonl` as the source of truth. Each poll,
for every capture:

- `Attributed` → ensure a correction exists (idempotent); if this capture is
  currently in its session quarantine file, **drain** it (remove that
  `source_line` from the file; delete the file when empty).
- `Unowned` → ensure it is quarantined (idempotent by `source_line`).
- `Pending` → skip.

So a quarantined capture drains exactly when re-derivation now yields
`Attributed` (a durable record covering its reign has since appeared). A capture
that stays `Unowned` forever stays in its quarantine file — terminal and
**countable** by simply counting quarantine rows. Add
`quarantine::drain(provider_dir, session_id, source_line)` and a small reader for
counting.

Rejected — **leave drained rows in place and compute "drained" as a derived
set.** Keeps quarantine append-only but makes the terminal count = quarantine
rows *minus* those with a correction — a join every reader must redo. Physically
draining keeps "terminal count = rows on disk" trivially honest. Quarantine is a
holding area, not published ledger, so rewriting it does not touch the
append-only ledger invariant.

## Decision 5 — Corrected view is the only token reader; gap is countable

**Chosen.** Add a pure fold in `lisa-core`:
`correct_usage(records) -> BTreeMap<ticket_id, TicketUsage>` where
`TicketUsage { tokens_in: Option<u64>, tokens_out: Option<u64>,
correction_count: usize }`. Rule per ticket: **if any correction rows exist, tokens
= sum of corrections** (the late-joined truth); **else fall back to the raw
authoritative Execution row's tokens** (legacy ledgers). A `capture-never` ticket
has no corrections and a null row → tokens stay `None`, never `0`.

- AC #4: `lisa status` gains a "Token usage" section reading *only* this corrected
  view — per-ticket joined totals plus aggregate.
- AC #3: the **gap** = authoritative `Done` tickets whose corrected `tokens_in`
  is still `None`. `lisa status` prints that count ("N completed tickets without
  joined usage yet"). Zero is never substituted for unknown.

Rejected — **surface on the dashboard (`ui.rs`) instead.** `lisa status` is the
plainer, unit-testable surface and already reads the ledger; the AC accepts
either. Dashboard wiring is deferred.

## Schema versioning

Adding a new row shape bumps `SCHEMA_VERSION` 8 → 9. Existing rows deserialize
unchanged (untagged enum, disjoint discriminator). Update
`docs/knowledge/provenance-ledger.md` (also correcting its stale "version 6").

## Scope guard

In: reign attribution rewrite, correction record + writer, sweep, quarantine
drain/count, corrected view, `lisa status` surface, doc + schema bump, tests. Out
(per story): new capture dimensions, cost computation, Claude cache-split,
historical backfill, dashboard wiring, session-per-ticket attribution.
