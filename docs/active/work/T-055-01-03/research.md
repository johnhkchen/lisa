# Research — T-055-01-03 · a-way-out-of-rejected

Descriptive map of the code the ticket names. No solutions proposed here.

## 1. What a completion is, and who owns each half

Three crates share the completion story and none of them holds all of it.

- `lisa-core/src/completion.rs` (1778 lines) — the pure reducer. `CompletionState`
  (`Eligible | Requested | CommandInFlight | Rejected{reason,retryability} | Confirmed`),
  `CompletionEvent`, `reduce()`, `reconcile()`, `CompletionGenerationId`
  (completion_id + attempt_id + generation, rendered `v1:<hex>:<hex>:<n>`),
  `CompletionSealReceipt` (`Commit{commit_id}` | `Journal{content_hashes}`). No I/O.
- `lisa-plugin/src/completion_journal.rs` (1847 lines, ~1058 non-test) — the durable
  record. Two halves that happen to live in one file:
  - **records + fold + append** (lines 1–472, 677–1058): `JournalRecord`,
    `JournalRecordBody`, `CompletionJournalTransition`, `CompletionJournalAggregate`,
    `load`, `append_with_seal`, `fold_bytes`, `apply_transition`, `matching_aggregate`.
    Depends only on `lisa_core::{completion, disposition, ticket, types}`, `serde_json`,
    and `crate::publication::RustPublication`.
  - **journal *seal*** (lines 474–674): `sha256`, `content_hash`, `collect_work_hashes`,
    `prepare_done_ticket`, `complete_with_journal_seal`. Needs `sha2` and
    `ticket::update_ticket_done`. This half is genuinely plugin-side.
- `lisa-plugin/src/lib.rs` (28088 lines) — the effect adapter. `dispatch_completion`,
  `execute_completion_effect`, `handle_completion_result`, `park_failed_completion`,
  `replay_in_flight_completion`, `reconciliation_state`. **Every journal write in the
  product goes through `State::journal_completion_transition` (lib.rs ~2397).**
- `lisa-cli/src/commit_transaction.rs` (1991 lines) — the plumbing beneath. `commit_ticket`,
  `complete_ticket`, `TransactionLock`, `AlternateIndex`, `discover_completion_commit`.
  It knows nothing about the journal, which is exactly the third gap in the ticket.

`lisa-plugin` **dev-depends on `lisa-cli`** (`features = ["test-support"]`), so plugin tests
already drive real transactions (`tests/hostile_order_regression.rs`, lib.rs ~12207, ~14413,
~23626, ~27554). `lisa-cli` depends only on `lisa-core`. There is no path from the CLI to the
journal writer today.

## 2. Precedent: lisa-core already reads the journal

`lisa-core/src/notes.rs` reads `.lisa/completion-journal.jsonl` through a narrow serde
projection (`CompletionJournalProjection`: `state`, `completion_id`, `attempt_id`,
`generation`, `note`) and `collect_notes` / `acknowledge_note` are called from both the
plugin (lib.rs ~9796) and the CLI (`lisa notes`). So core reading the journal is established.
Core *writing* it is not.

## 3. Why `rejected` has no exit today

### 3.1 The two coercions that re-arm a failed completion forever

`reconciliation_state` (lib.rs 2436–2469) rewrites the stored aggregate state before every
decision. Two arms matter:

```rust
|| (matches!(aggregate.state(), Rejected { retryability: ActionRequired, .. })
    && durable_ticket.is_some_and(|t| t.status == TicketStatus::Open))
=> CompletionState::Eligible
```

`lisa unblock` sets `status: open` (unblock.rs:77). From that moment every reconcile pass
re-derives `Eligible`, so `reconcile()` returns `Effect(LaunchCompletion)` again. Nothing
counts these re-arms.

The second coercion (lib.rs 2672–2680) is unconditional for operator input:

```rust
CompletionState::Rejected { retryability: ActionRequired, .. }
    if matches!(source, CompletionSource::OperatorRequested(_)) => CompletionState::Eligible
```

`MarkDoneKey` is the only `OperatorRequestSource` (lib.rs 659–662); it arrives via
`request_operator_completion` (lib.rs 9088) from `[d]` / the desk signature / the reason step.
So `[d]` on an action-required rejection always restarts, without limit.

### 3.2 What the restart runs into

`execute_completion_effect` mints a **new** key — `CompletionGenerationId(ticket, "operator",
generation)` — and `build_completion_command` shells out to `lisa complete-ticket`.
`complete_ticket` → `commit_ticket_with_key(…, Some(key))` → `discover_completion_commit`
greps history for `Lisa-Completion-Key: <key>` (commit_transaction.rs 1025–1072). A *new*
generation key is never in history, so discovery misses, the alternate index stages nothing,
and line 831 returns `ticket <id> has no changes in the requested include paths`.

This is the load-bearing detail for this ticket: **T-055-01-02 makes an empty diff succeed only
when a commit carrying *this* key is at HEAD.** A recovery that mints a fresh generation still
fails after that fix. Convergence-by-same-key rescues reconciliation replays
(`ReplayCommandInFlight` reuses the stored key, lib.rs 2962–3011); it does not rescue a new
operator request.

### 3.3 Bounded per attempt, unbounded across attempts

`MAX_COMPLETION_FAILURES = 2` (lib.rs:90). `classify_completion_failure` (468) maps the error
text to a `CompletionFailureClass`; `completion_failure_action` (501) picks Retry /
WaitForDeadline / Park. `has no changes in the requested include paths` matches none of the
patterns, so it classes as `Unrecognized` → **Park immediately**. `park_failed_completion`
journals `Rejected{ActionRequired}`, writes a `block` disposition, restores the prior phase,
sets `status: blocked`, releases the slot and thread.

The bound is per *generation*. `apply_transition`'s `Requested` arm (completion_journal.rs
806–818) resets state to `Eligible` and rebuilds the aggregate with `failure_count: 0`
whenever the key differs — so a new generation starts the budget over. Nothing in the journal
counts generations that ended action-required.

### 3.4 The escapes the operator actually has

- `lisa unblock <id>` (unblock.rs 41–82) — requires `status: blocked`, runs the disposition's
  optional read-only `check`, flips to `open`, leaves phase alone. Its whole contract is
  "verify what changed and let a waiting ticket run again". §3.1 shows why this re-enters the
  failing phase. AC5 protects this behavior for tickets where it works.
- `lisa proposal dismiss <id>` (proposal.rs:62) — `"{ticket_id} isn't waiting."` once the
  board no longer reads blocked.
- `[d]` in the dashboard, with the E-053 override catalog (`lisa-core/src/operator_override.rs`:
  `OverrideReason::{EvidenceSatisfies, CannotVerifyHere, BeyondTicketReach, NoReviewOnFile}`).
  Signs a *note*, then runs the same failing transaction.
- `lisa complete-ticket` invoked by hand — commits, returns a commit id, writes **no journal
  record**, because journaling lives above it in the adapter.

## 4. The journal, and why hand-appending is not an option

`fold_bytes` is strict and `restore_completion_journal` (lib.rs 2375) is fail-closed: a load
error sets `completion_journal_healthy = false`, and `schedule_ready_tickets` early-returns on
that flag (lib.rs 5256). A single unreplayable line stops all scheduling with one Error line in
the feed. This is a recorded field incident (starfox, 2026-07-19): a hand-written `confirmed`
after `rejected` under the same key bricked a board.

`apply_transition`'s `Confirmed` arm goes through `reduce()`, and `reduce()` refuses
`CommandSucceeded` from `Rejected` (`UnexpectedEvent`). So **a terminal success cannot be
recorded against a rejected key**. The only legal route from `Rejected` to `Confirmed` in the
existing format is a *new generation*: `Requested` (which the `Requested` arm resets to
`Eligible` when the key differs) → `CommandInFlight` → `Confirmed`. No new record types are
needed for that; the format E-042 fixed already permits it.

Also durable and relevant:
- `CompletionJournalAggregate` carries `failure_count`, `failure_limit`, `retries_exhausted`,
  `prior_phase`, `prior_status`, `completion_note`, `confirmed_receipt`.
- `masks_durable_done()` returns true for `Requested | CommandInFlight |
  Rejected{ActionRequired}` — `mask_completion_transaction` (2418) rewrites a scanned Done
  ticket back to its prior phase/status while a completion is unresolved. This is why "board
  and journal disagree" is visible: the ticket file can say Done and the board still shows
  Review.
- `retries_exhausted` gates `replay_in_flight_completion` (2986) and the reconcile replay path
  (2585).

## 5. The disposition that conflates transport with verdict

`park_failed_completion` (lib.rs 3080–3205) is the only writer of a machine-authored
disposition:

```rust
serde_json::json!({
    "disposition": "block",
    "reason": technical_reason.clone(),      // <- the failed command's stderr
    "remedy_owner": "operator",
    "ask": structured_ask,
})
```

`technical_reason` is built in `handle_completion_result` (3473):
`"Completion commit failed for {ticket} authority {:?} ({:?}, exit {:?}): {stderr}"`. When the
class is `Unrecognized`, `completion_failure_ask` returns `None` and the document degrades to
`{"disposition":"block","reason":"<stderr>"}` — bit-for-bit the field artifact quoted in the
ticket.

Downstream readers cannot tell this apart from a reviewer's block:
- `lisa-core/src/disposition.rs` — `ReviewDisposition::Block { reason, remedy_owner, ask,
  steps, check, unstructured }`. `unstructured: true` marks "missing remedy structure", not
  "written by a machine". `parse_review_disposition` is tolerant; `check_review_disposition`
  is the strict authoring check and **rejects unknown fields** (`"remove extra block fields"`,
  disposition.rs 263–268).
- `lisa-core/src/parking.rs::collect_parked_remedies` projects Block into `ParkedRemedy`
  (`ticket_id, remedy_owner, ask, reason, steps, check, proposal`) — the single feed for
  `lisa status`, the dashboard, and `lisa unblock`'s remedy lookup.
- `ReviewDisposition::Block` is constructed at 27 sites across 7 files (mostly tests).

Note the path split: agents write `.lisa/attempts/<id>/<gen>/work/review-disposition.json` and
run `lisa check-disposition`; `park_failed_completion` writes the *canonical*
`<work_dir>/<id>/review-disposition.json`. `check-disposition` never reads the canonical copy.

## 6. Evidence available for "the work is verifiably at HEAD"

`commit_transaction.rs` owns the only key-in-history reader:
`COMPLETION_KEY_PREFIX = "Lisa-Completion-Key: "` (979), `completion_key_marker` (981),
`discover_completion_commit` (1025) — `git log --format=%H --fixed-strings --grep <marker>`
then a `git show -s --format=%B` re-verification per candidate, with an unborn-HEAD guard.
It is private and matches one exact key.

So the facts a recovery step could stand on, in decreasing strength:
1. A commit reachable from HEAD carrying `Lisa-Completion-Key:` for **this ticket's**
   completion id (any attempt, any generation) — proof that some completion for this ticket
   sealed. Keyed evidence, per the epic's N3 constraint.
2. The ticket file at HEAD reading `status: done` / `phase: done`.
3. An empty include-path diff — explicitly *not* evidence (N3).

## 7. Constraints this ticket inherits

- **File ownership.** T-055-01-02 runs in parallel and owns
  `crates/lisa-cli/src/commit_transaction.rs`. The story states the two repairs "share no
  file". Anything this ticket needs from that file must be obtained without editing it.
- **PRESERVE (E-041/E-042).** The reducer's shape does not change. Journal format, correlation
  ids and generation keys stay as they are. Adding a *derived projection* to
  `CompletionJournalAggregate` is not a format change; adding a record variant would be.
- **AC5.** `lisa unblock`'s meaning for ordinary parked tickets is unchanged.
- **Out of slice** (story): changing what unblock means, locking redesign, changes to how the
  review phase authors dispositions, the archive/retire workflow.
- **N2.** Recovery must be bounded and end in a *named, actionable* state.
- The recovery must work with the loop **stopped**. In the field the operator's escape came
  after killing a loop that would not stop re-attempting; a route that needs a healthy running
  plugin is not a route out.

## 8. Test fixtures already in place

- `crates/lisa-plugin/src/tests/operator_recovery_matrix.rs` — `review_state()`,
  `add_active_review_attempt`, `submit_from_done_key`, `assert_operator_pending`. The natural
  home for a bounded-`MarkDoneKey` test.
- `crates/lisa-cli/tests/guard_waits_its_turn.rs` — T-055-01-01's concurrency fixture.
- `crates/lisa-cli/tests/parked_ux.rs` — `lisa unblock` behavior, the AC5 regression net.
- `crates/lisa-cli/tests/help_surface.rs` — asserts the exact command list and help text; any
  new subcommand must be added there.
- `crates/lisa-plugin/src/lib.rs` ~11416, ~17104 — helpers that append journal transitions
  directly in tests.
- `just check` = fmt + clippy + WASM check + workspace tests.
