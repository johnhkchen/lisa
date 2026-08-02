# Design — T-055-01-03 · a-way-out-of-rejected

Three behaviors, decided one at a time, each grounded in the Research map.

---

## Decision 1 — where the recovery route lives

### The constraint that settles it

The field operator reached `rejected` **and then killed the loop**, because the loop would not
stop re-attempting. A recovery route that requires a healthy running plugin asks the operator
to keep running the thing that is broken. So the route must work with the loop stopped.

That rules out every plugin-mediated option and forces the question: the journal is written
only by `State::journal_completion_transition`, and the CLI cannot reach it.

### Options

**A. Plugin-side action (a new dashboard key, or `[d]` learning to reconcile).**
No new crate boundary; reuses `admit_operator_completion` and the E-053 override catalog.
Rejected: needs the loop running, and AC1 asks for a *command*.

**B. CLI writes a durable recovery request; the plugin converts it on its next poll.**
Keeps journal authorship in the adapter, which is architecturally tidy. Rejected: the command
alone does not leave board and journal in agreement — it leaves an IOU. AC1 says the single
command produces the terminal record. Same loop-must-be-running problem.

**C. CLI hand-appends a `confirmed` row to `.lisa/completion-journal.jsonl`.**
Rejected outright. `fold_bytes` is strict and `restore_completion_journal` is fail-closed; one
unreplayable line fences *all* scheduling with a single Error line in the feed. This is a
recorded field incident (starfox, 2026-07-19). A recovery command that can brick the board is
not a recovery command.

**D. Move the journal's record/fold/append half into `lisa-core`; the CLI writes through the
same validated fold.**
Chosen.

### Why D

- `lisa-core` **already reads** the journal (`notes.rs`), so core knowing the journal is not a
  new dependency direction — only the write half moves.
- The fold-before-append discipline is the whole safety property. Reusing it means the
  recovery command validates the entire history before it writes a byte: an unhealthy journal
  makes the command *fail loudly* rather than append a line the plugin will choke on. C's
  failure mode becomes structurally impossible.
- The split is clean along an existing seam. The file already has two halves: records/fold
  (depends on `lisa_core::*` + `serde_json` only) and the journal *seal* (`sha2`,
  `update_ticket_done`, `RustPublication`). Only the first moves.
- Cost: a large mechanical diff and one API shape change. `append_with_seal` needs an atomic
  publish, which lives in the plugin's `publication.rs`. Rather than duplicate that mechanism
  in core, core exposes `append_with_seal_using(path, seal, transition, publish)` taking the
  writer as a closure — the same pattern `complete_with_journal_seal_and_publish` already uses
  in this very file. The plugin passes `RustPublication`; the CLI passes its own temp+rename.
  Plugin call sites keep their current signature through a one-line wrapper.

**Rejected sub-option:** duplicating a minimal record shape in core (the way `notes.rs`
projects). A second definition of the record format is exactly the drift E-042 exists to
prevent, and a writer built on a projection cannot validate what it writes.

---

## Decision 2 — what the recovery command does, and what proves it

### Name

`lisa already-done <TICKET_ID>` — "Finish a ticket whose work is already recorded in history."
A thing you could say aloud at a kitchen table. Not `reconcile`, not `seal-recovered`.

### Rejected: teaching `lisa unblock` to reconcile

The ticket offers it. Rejected because unblock's contract is "verify what changed and let a
waiting ticket run again" — a *scheduling* flip. Recovery is a *terminal record*. Folding them
would mean unblock sometimes returns a ticket to Review and sometimes finishes it, decided by
journal state the operator cannot see. AC5 asks that unblock's meaning not be repurposed; the
cleanest way to honor that is a second, differently-named command.

Unblock still changes in one direction only: it **declines** for a ticket past the recovery
bound and names `lisa already-done`. Declining a case it cannot fix is behavior unblock
already has (`"That didn't work yet — …"`), not a new meaning.

### The evidence

N3 is the governing constraint: emptiness is never proof; the key is.

`already-done` accepts exactly one fact — **a commit reachable from HEAD whose message carries
`Lisa-Completion-Key: v1:<hex ticket-id>:` for this ticket** (any attempt, any generation).
That is proof that some completion transaction for this ticket sealed. Nothing else counts:
not a Done frontmatter in the working tree, not an empty diff, not the operator's assertion.

Why *any* generation rather than the rejected key's own: the rejected generation is precisely
the one that never committed. The commit that exists carries an earlier key (the field run's
two hand-run `complete-ticket` invocations, at generations 2 and 3). Requiring the rejected
key would make the command refuse in the only situation it exists for.

Why not also require the ticket to read Done at HEAD: the ticket bytes are Lisa's to write and
the command writes them; making them a precondition would refuse the case where the operator's
commit predates the mask.

### The refusals, each named

| Situation | Response |
|---|---|
| Ticket not on the board | `I couldn't find <id>.` |
| No journal record for it | `Lisa has no record of trying to finish <id>.` |
| State is `Confirmed` | `<id> is already finished.` |
| State is anything other than `Rejected{ActionRequired}` | `<id> isn't stuck — Lisa is still working on it.` |
| Seal is `journal` | `This project records finished work in the journal, not in commits.` |
| No keyed commit reachable from HEAD | `I can't find <id>'s finished work in this repository's history. Nothing changed.` |

The last row is the negative fixture. A naive "the operator says so, so mark it done"
implementation fails it.

### The transition sequence

`reduce()` refuses `CommandSucceeded` from `Rejected` — a terminal success **cannot** be
recorded against a rejected key, and forcing one is the starfox brick. The legal route already
exists in the format E-042 fixed: a new generation.

```
Requested   { key: (ticket, "operator", prior_generation + 1), prior_phase, prior_status }
CommandInFlight { key, correlation = key.to_string(), deadline = now }
Confirmed   { key, correlation, receipt: Commit { commit_id } }
```

`apply_transition`'s `Requested` arm already resets a `Rejected` aggregate to `Eligible` when
the key differs, and `retryable_rejection_can_start_another_request_generation` already pins
that behavior. **No new record type, no schema bump, no format change** — E-042 preserved.

Each of the three appends re-folds the whole file, so the command is safe to interrupt: a
partial sequence leaves the aggregate in a state the plugin can replay, never an illegal one.

Then the ticket file is written Done (`ticket::update_ticket_done`) so the board agrees. The
command does **not** commit that write — committing here would need the isolated transaction
in a file another in-flight ticket owns, and would risk clobbering concurrent work. It reports
that the ticket file changed so the operator can commit it with the rest.

---

## Decision 3 — bounding the re-attempt

### Where the unbounded loop actually is

Two coercions, both in `lib.rs`, both turning `Rejected{ActionRequired}` into `Eligible`:

1. `reconciliation_state` (2445–2451) — whenever the ticket's status is `open`. `lisa unblock`
   sets `open`. Every reconcile pass thereafter re-requests. **This is the "re-attempts on
   every loop start" the epic describes.**
2. `dispatch_completion` (2672–2680) — unconditionally for `OperatorRequested`. This is `[d]`.

Neither counts anything.

### The counter

`CompletionJournalAggregate` gains a derived field `action_required_generations: u8`,
incremented when a transition lands `Rejected{ActionRequired}` and **carried across** the
`Requested` reset that starts a new generation. Derived projection, not a record field: the
journal format is untouched and old journals fold to the same number.

Bound: `MAX_ACTION_REQUIRED_GENERATIONS = 2`, sitting beside the existing
`MAX_COMPLETION_FAILURES = 2`. Two generations × two command attempts is four real tries — the
field run's shape, and cheap.

Rejected alternative: a wall-clock or loop-start counter. Not durable across restarts, and the
epic's complaint is specifically "bounded per attempt but unbounded across loop starts" — the
bound must live in the same durable place the attempts do.

### The named state at the bound

Both coercions become conditional on `action_required_generations < MAX`. Past it:

- `reconcile()` returns `None` — the loop stops re-attempting.
- `[d]` gets `reduce()`'s typed refusal, logged as a `CompletionRejected` event whose detail
  names `lisa already-done <id>`.
- The park that *reaches* the bound writes an ask that names the state and the command:
  `Run `lisa already-done <id>` if this ticket's work is already saved in history.` — one
  short lead sentence with an action verb, which is what `parking.rs::validate_block_ask`
  requires.
- `lisa unblock <id>` declines with the same pointer, so the ticket stays `blocked`.
- The dashboard's send-back `[s]` declines likewise.

Staying `blocked` is what stops the seat and the pane: blocked tickets are not scheduled, and
`park_failed_completion` already releases the slot and removes the thread. The leak was
unblock flipping to `open` and the scheduler starting a fresh review attempt for a ticket
whose completion could never succeed.

---

## Decision 4 — separating a transport failure from a verdict

`park_failed_completion` writes the only machine-authored disposition, and today its `reason`
is the failed command's stderr. The operator read `block` as a verdict on twelve recipes.

### Chosen: one parsed field on the block

`lisa-core::disposition` gains

```rust
pub enum DispositionOrigin { Review, InternalCommand }
```

and `ReviewDisposition::Block` gains `origin: DispositionOrigin`. The tolerant parser reads an
optional `"origin"` field (`"internal-command"` → `InternalCommand`, absent → `Review`,
anything else → the existing unstructured operator fallback). `ParkedRemedy` carries it
forward so `lisa status` and the dashboard can label the two apart.

The strict authoring check (`check_review_disposition`) is **not touched**: it already rejects
unknown fields, so an agent cannot author `origin` and claim a transport failure — and the
story puts changes to review-phase disposition authoring out of slice.

### And the reason stops carrying the error

`park_failed_completion`'s document becomes:

```json
{"disposition":"block",
 "origin":"internal-command",
 "reason":"Lisa could not record <id>'s finished work. This is a recording failure, not a judgement about the work. The exact error is in .lisa/completion-journal.jsonl.",
 "remedy_owner":"operator",
 "ask":"<structured ask>"}
```

The raw command text is not dropped — it is already durable in the journal's `FailureObserved`
and `Rejected` rows and in the activity log, both of which this ticket leaves alone. Moving it
out of `reason` is the point: `reason` is what `lisa status` shows a person, and it should be a
statement about the boundary that failed, not a git error presented as a review finding.

Consequence: `CompletionFailureClass::Unrecognized` can no longer fall back to "put the raw
failure in the ask" (`completion_failure_ask` returns `None` today). It gets a real ask — the
`lisa already-done` pointer when the bound is reached, and otherwise the existing
`lisa unblock` pointer.

### Rejected alternatives

- **A separate `command_error` field.** Adds a field nothing parses. The detail already has two
  durable homes.
- **A new `ReviewDisposition` variant.** Would change `authorizes_completion` and every match
  arm across three crates for a distinction that is one boolean.
- **Distinguishing by prose** (a `[transport]` prefix in `reason`). Exactly what the AC forbids:
  "by field, not by reading the prose".

---

## What this design deliberately does not do

- Does not touch `crates/lisa-cli/src/commit_transaction.rs` — T-055-01-02 owns it. The
  key-in-history reader `already-done` needs is written fresh in a new CLI module against a
  marker helper added to `lisa-core::completion` (the natural home for
  `CompletionGenerationId`'s own commit-trailer rendering). Drift between the new reader and
  the existing private `discover_completion_commit` is caught by a test that commits through
  the real `complete_ticket` and then finds that commit with the new reader.
- Does not change the reducer in `lisa-core/src/completion.rs`.
- Does not change the journal schema, correlation ids, or generation keys.
- Does not change what `lisa unblock` does for tickets it can already fix.
- Does not add an archive/retire workflow.
