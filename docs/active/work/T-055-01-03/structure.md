# Structure — T-055-01-03 · a-way-out-of-rejected

The blueprint. Files, boundaries, interfaces, ordering. Not code.

## Ownership fence

`crates/lisa-cli/src/commit_transaction.rs` is **not** in this ticket's include set —
T-055-01-02 owns it. Everything below is written around that file, never into it.

---

## A. `lisa-core` — the shared journal and the two new vocabulary items

### A1. `crates/lisa-core/src/completion.rs` — modified

Add the commit-trailer rendering next to the key it renders, so a second crate can read the
trailer without owning the transaction.

```rust
pub const COMPLETION_KEY_PREFIX: &str;                       // "Lisa-Completion-Key: "
pub fn completion_key_marker(key: &CompletionGenerationId) -> String;
pub fn completion_key_ticket_prefix(completion_id: &CompletionId) -> String;
```

`completion_key_ticket_prefix` renders `Lisa-Completion-Key: v1:<hex completion-id>:` — the
prefix every generation for one ticket shares. The private `write_hex` formatter is refactored
to sit on a shared `fn hex(bytes: &[u8]) -> String`, with `Display` unchanged in output.

No change to `CompletionState`, `CompletionEvent`, `reduce`, `reconcile`, or any receipt type.

### A2. `crates/lisa-core/src/completion_journal.rs` — new (moved)

Receives, verbatim except for visibility and the publish seam, lines 1–472 and 677–1058 of
`crates/lisa-plugin/src/completion_journal.rs`:

| Item | Was | Becomes |
|---|---|---|
| `CompletionJournalTransition` | `pub(crate)` | `pub` |
| `CompletionFailureClass`, `FailureConsequence` | `pub(crate)` | `pub` |
| `CompletionJournalAggregate` + accessors | `pub(crate)` | `pub` |
| `JournalRecord`, `JournalRecordBody`, `JournalRetryability` | private | private (unchanged) |
| `load` | `pub(crate)` | `pub` |
| `fold_bytes`, `apply_transition`, `matching_aggregate`, `generation_key` | private | private |
| `append_with_seal` | `pub(crate)`, publishes via `RustPublication` | replaced, see below |

New public surface:

```rust
pub const COMPLETION_JOURNAL_RELATIVE_PATH: &str;   // ".lisa/completion-journal.jsonl"

pub fn append_with_seal_using<F>(
    path: &Path,
    seal: CompletionSeal,
    transition: CompletionJournalTransition,
    publish: F,
) -> Result<CompletionJournalAggregate, String>
where F: FnOnce(&Path, &[u8]) -> Result<(), String>;
```

The closure receives the destination and the **complete new file bytes**; core keeps the
fold-validate-then-hand-off order exactly as it is today, so validation still precedes any
write. This mirrors the seam `complete_with_journal_seal_and_publish` already uses in the same
file, rather than duplicating `RustPublication` in core.

Also new, and the only behavioral addition in this module:

```rust
impl CompletionJournalAggregate {
    /// Generations of this completion that ended action-required, counted
    /// across generations. Derived from the existing records; no new record
    /// type and no schema change.
    pub fn action_required_generations(&self) -> u8;
}
```

Maintained in `apply_transition`: the `Rejected` arm increments it (saturating) when the
resulting retryability is `ActionRequired`; the `Requested` arm **carries the prior value
forward** when it resets a `Rejected`/`Confirmed` aggregate to `Eligible` for a new key, and
starts at 0 for a first-ever key. Old journals fold to the same number, so nothing on disk
changes meaning.

Tests moved here from the plugin (they exercise private fold internals): everything from
`requested_in_flight_and_confirmed_reconstruct_after_each_restart` through
`new_rows_carry_the_pinned_seal_and_mixed_generations_fail_closed`, plus a new
`action_required_generations_survive_a_new_key_and_stop_at_the_bound`.

### A3. `crates/lisa-core/src/disposition.rs` — modified

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DispositionOrigin { Review, InternalCommand }
```

`ReviewDisposition::Block` gains `origin: DispositionOrigin`.

- `validate_block_structure` (tolerant path) reads an optional `"origin"` key:
  `"internal-command"` → `InternalCommand`; absent → `Review`; any other value → falls to
  `unstructured_block`, which is `Review` + operator fallback. Fail-safe direction: an
  unreadable origin never claims to be a machine failure.
- `unstructured_block` sets `origin: Review`.
- `check_review_disposition` / `check_block_document` are **untouched**. Their existing
  "remove extra block fields" rule already refuses an agent-authored `origin`, which is the
  behavior we want, and review-phase authoring is out of this slice.

### A4. `crates/lisa-core/src/parking.rs` — modified

`ParkedRemedy` gains `pub origin: DispositionOrigin`; `collect_parked_remedies`'s
destructuring of `Block` carries it through. No filtering change — a recording failure is
still a park an operator must clear.

### A5. `crates/lisa-core/src/lib.rs` — modified

One line: `pub mod completion_journal;`.

---

## B. `lisa-plugin` — the bound, the honest disposition, the thinner journal module

### B1. `crates/lisa-plugin/src/completion_journal.rs` — modified (shrinks)

Keeps only the journal **seal** half: `sha256`, `completion_content_path`, `content_hash`,
`collect_work_hashes`, `prepare_done_ticket`, `complete_with_journal_seal_and_publish`,
`complete_with_journal_seal`, and their tests.

Adds a re-export block so the ~10 call sites in `lib.rs` keep compiling unchanged:

```rust
pub(crate) use lisa_core::completion_journal::{
    load, CompletionFailureClass, CompletionJournalAggregate, CompletionJournalTransition,
    FailureConsequence,
};

pub(crate) fn append_with_seal(path, seal, transition) -> Result<CompletionJournalAggregate, String>
```

— the wrapper supplies the `RustPublication` closure. `publication.rs` is not touched.

### B2. `crates/lisa-plugin/src/lib.rs` — modified

**b2.1 The bound.** New constant beside `MAX_COMPLETION_FAILURES`:

```rust
const MAX_ACTION_REQUIRED_GENERATIONS: u8 = 2;
```

New predicate on `State`:

```rust
fn recovery_generations_exhausted(&self, ticket_id: &str) -> bool
```
— true when the ticket's aggregate reports
`action_required_generations() >= MAX_ACTION_REQUIRED_GENERATIONS`.

Two coercion sites become conditional on it being false:
- `reconciliation_state` (~2445) — the `Rejected{ActionRequired}` + `status: open` arm.
- `dispatch_completion` (~2672) — the `OperatorRequested` arm.

**b2.2 The named exit.** `completion_failure_ask` gains a caller-supplied "exhausted" input
(signature becomes `completion_failure_ask(class, ticket_id, exhausted: bool)`), and every
class returns `Some(..)` when exhausted:

> ``Run `lisa already-done <id>` if this ticket's work is already saved in history.`` followed
> by one context sentence.

The lead sentence is ≤160 characters, single-line, and contains `Run` — satisfying
`parking::validate_block_ask`. `CompletionFailureClass::Unrecognized` also gains a
non-exhausted ask (it returns `None` today, which is what let the raw stderr become the ask).

**b2.3 The honest disposition.** `park_failed_completion` writes:

```json
{"disposition":"block",
 "origin":"internal-command",
 "reason":"Lisa could not record <id>'s finished work. This is a recording failure, not a judgement about the work. The exact error is in .lisa/completion-journal.jsonl.",
 "remedy_owner":"operator",
 "ask":"<ask from b2.2>"}
```

`technical_reason` still goes to the journal (`Rejected`/`FailureObserved` rows) and the
activity log — both unchanged. It simply stops being the disposition's `reason`.

**b2.4 Send-back declines past the bound.** `send_back_for_review` (~8983) gains one guard:
when `recovery_generations_exhausted`, log a warning naming `lisa already-done <id>` and
return without flipping to `open`. Keeps the seat and pane free.

**b2.5 Mechanical.** `ReviewDisposition::Block` construction/destructuring sites in this file
gain `origin`.

### B3. `crates/lisa-plugin/src/tests/operator_recovery_matrix.rs` — modified

New tests:
- `repeated_done_key_stops_at_the_bound_and_names_the_command` — AC2. Drives `[d]` past
  `MAX_ACTION_REQUIRED_GENERATIONS`, asserts no further effect launches, asserts the ticket is
  `blocked`, asserts the disposition's ask contains `lisa already-done`, asserts the slot and
  thread are released.
- `a_recording_failure_is_not_a_reviewers_block` — AC3.

---

## C. `lisa-cli` — the command

### C1. `crates/lisa-cli/src/already_done.rs` — new

The whole recovery route, with no dependency on `config` so the library build can export it.

```rust
pub struct AlreadyDoneRequest<'a> {
    pub project_root: &'a Path,   // repository / project root
    pub ticket_dir: &'a Path,
    pub journal_path: &'a Path,
}

pub enum AlreadyDoneOutcome {
    Recovered { ticket_id: String, commit_id: String, ticket_file_rewritten: bool },
    Declined(String),
}

pub fn run_already_done(request: AlreadyDoneRequest, ticket_id: &str)
    -> Result<AlreadyDoneOutcome, String>;
```

Internal steps, in order, each with its own named decline (see design.md's refusal table):

1. `ticket::scan_tickets` → locate the ticket.
2. `lisa_core::completion_journal::load` → the aggregate; require
   `Rejected { retryability: ActionRequired, .. }` and `seal == Commit`.
3. `find_sealed_commit(project_root, completion_id) -> Result<Option<String>, String>` —
   private. `git log --format=%H --fixed-strings --grep <ticket prefix>` from HEAD, guarded for
   an unborn HEAD, then `git show -s --format=%B <sha>` per candidate and a line-prefix
   re-verification. Deliberately the same two-step shape as the transaction's private
   `discover_completion_commit`, built on `lisa_core::completion::completion_key_ticket_prefix`
   so the trailer text has one definition.
4. Mint `CompletionGenerationId(completion_id, AttemptId("operator"), prior_generation + 1)`.
5. Append `Requested` → `CommandInFlight` → `Confirmed{Commit{commit_id}}` through
   `append_with_seal_using`, with a private `atomic_write` closure (sibling temp + rename,
   matching `proposal.rs::atomic_write`).
6. `ticket::update_ticket_done` when the file is not already Done; report whether it changed.

### C2. `crates/lisa-cli/src/unblock.rs` — modified

`run_unblock` gains one early decline, after the "isn't waiting" guard and before the remedy
check: if the journal's aggregate for this ticket reports the generations exhausted, return
`Declined` naming `lisa already-done <id>`. Ordinary parked tickets — the ones with no
aggregate, or with one under the bound — take the existing path byte for byte (AC5).

`run_world_rechecks` is untouched.

### C3. `crates/lisa-cli/src/main.rs` — modified

New visible operator subcommand between `unblock` and `doctor`:

```
already-done  Finish a ticket whose work is already recorded in history
  Usage: lisa already-done <TICKET_ID> [--path <PATH>]
  Example: lisa already-done T-001 --path ./my-project
```

`display_order` renumbers: unblock 4, already-done 5, doctor 6, proposal 7, loop 8.
Dispatch resolves config, builds `AlreadyDoneRequest`, prints the outcome; `Declined` prints to
stderr and exits 1 (matching `unblock`'s convention).

### C4. `crates/lisa-cli/src/lib.rs` — modified

`#[cfg(feature = "test-support")] pub mod already_done;` — the existing `capture_usage`
pattern, so plugin tests can drive the real command.

### C5. `crates/lisa-cli/src/check_disposition.rs`, `crates/lisa-cli/src/proposal.rs` — modified

Mechanical: `ReviewDisposition::Block` patterns gain `origin`.

---

## D. Tests that pin the acceptance criteria

| File | Status | Pins |
|---|---|---|
| `crates/lisa-plugin/src/tests/rejected_has_an_exit.rs` | new | **AC1** — the full field sequence: generation 1 seals a real commit through `lisa_cli::commit_transaction::complete_ticket`; generation 2 fails with the empty-include-path error and parks action-required; `lisa_cli::already_done::run_already_done` recovers; asserts the journal's final state is `Confirmed` with that commit id and the ticket reads Done. Also **AC4** — a direct `complete_ticket` still works as plumbing and still writes no journal row, while the supported route leaves both in agreement. |
| `crates/lisa-cli/tests/already_done.rs` | new | The negative fixture: rejected with **no** keyed commit in history declines, changes nothing, and exits non-zero. Plus the wrong-state and journal-seal declines. |
| `crates/lisa-plugin/src/tests/operator_recovery_matrix.rs` | modified | **AC2**, **AC3** (see B3). |
| `crates/lisa-cli/tests/parked_ux.rs` | modified | **AC5** — existing cases unchanged; one new case for the exhausted decline. |
| `crates/lisa-cli/tests/help_surface.rs` | modified | 17→18 own commands, 8→9 operator commands, top-level and per-command snapshots, jargon check. Test fn renamed. |
| `crates/lisa-core/src/completion.rs` tests | modified | The trailer literals. |
| `crates/lisa-core/src/completion_journal.rs` tests | moved + new | Fold behavior; the generation counter. |
| `crates/lisa-core/src/disposition.rs` tests | modified | `origin` parsing in both directions; the strict check still refuses it. |
| `crates/lisa-core/tests/completion_state_machine.rs` | modified | Mechanical `Block` construction. |

## E. Ordering

The move must land first — everything else compiles against it.

1. **A1** trailer helpers (self-contained, green on its own).
2. **A2 + A5 + B1** the journal move (mechanical; workspace must be green before anything
   behavioral rides on it).
3. **A3 + A4 + C5 + B2.5** the `origin` field, mechanically threaded (green).
4. **B2.3** the honest disposition + **B3** its test. (AC3 closed.)
5. **A2's counter + B2.1/B2.2/B2.4** the bound + **B3** its test. (AC2 closed.)
6. **C1 + C3 + C4** the command + **C2** unblock's decline. (AC1, AC4, AC5.)
7. **D** the field-sequence and negative-fixture tests, then `just check`.

Steps 1–3 are behavior-preserving and each is independently committable. Steps 4–6 each close
one acceptance criterion.
