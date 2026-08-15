# T-070-01-04 — a ticket whose completion was rejected can now be finished

## What was actually wrong

`already-done` read *"the work is already recorded in history"* as *"the seal is
already in history"*. Those are the same thing only when the completion commit
succeeded and its result was lost on the way back — the lost-race case the
command was built for. The `renderer` field case was the other half: the
**completion commit itself failed**, so no commit anywhere carried
`Lisa-Completion-Key:`, and the one command documented as the way out refused on
a ticket nothing else could finish either.

Everything else in the ticket follows from that single blind spot being silent:
the run declined the ticket on every pass and recorded the decline where nobody
looks, the board went on calling it ready, and the journal row that decided a
person must intervene did not say what the person should do.

## The six commits

| Commit | What it does |
| --- | --- |
| `6e168f4` | `park_failed_completion` computes the ask **before** journaling and writes it into the Rejected row's `reason`, so `retryability: action-required` sits beside the command that clears it. |
| `3756f18` | `already-done` writes the seal when the seal is the only thing missing. |
| `993d70d` | A scheduling pass says once, in the Activity pane, why it will not take a ready ticket. |
| `dee294c` | `lisa status` stops counting a ticket ready that no run will take. |
| `aa847f2` | The help text and README say what the command actually does. |
| `75c44eb` | The blocked count is re-derived rather than incremented, and the refusal names a path the way an operator would type it. |

## The five acceptance criteria, one by one

**A documented command finishes it — `lisa already-done`.** When the journal
holds a rejected/action-required completion and no commit carries the ticket's
key, the command now runs the *same* completion transaction the loop would have
(`commit_transaction::complete_ticket`) under an `operator` generation key, then
journals Requested → CommandInFlight → Confirmed against the commit it just
made. The commit comes before the journal rows on purpose: a run killed in
between leaves the key in history, which is exactly the state the adopting arm
already settles, so re-running finishes it.

**Say what it looks for.** In order: a commit carrying
`Lisa-Completion-Key: v1:<ticket>:…`, and failing that the ticket's published
review at `docs/active/work/<id>/review.md`. The refusal now prints both, and
says outright that a commit naming the ticket in its subject line is neither —
which is the exact belief that made the field refusal read as a bug in `git log`.
The same two are in `lisa already-done --help` and the README.

**What it will not do.** It still refuses your word. It also refuses to step
over a *reviewer's* block — but not over Lisa's own `origin: internal-command`
block, which is the recording-failure note the park writes and therefore the
state the command exists to clear. That distinction is the one T-048-01 put in
the disposition for exactly this reason.

**A run says why.** `DurableDoneMasked` split into a second reason,
`CompletionUnsealed`. Every other decline in that loop is transient and stays
recorded rather than logged — this one never clears on its own, so it gets one
`Warning` in the Activity pane naming the ticket and `lisa already-done <id>`.
Once, guarded by `unsealed_ready_reported`, cleared when the ticket comes back
round schedulable.

**The board agrees with itself.** `lisa status` folds the completion journal,
removes those tickets from `ready[]`, re-derives `blocked` by the DAG's own
subtraction, and prints **Tickets no run will take** with the ticket and the
command. The condition is the scheduler's own `masks_durable_done()`, so the two
cannot drift. `--json` gains `unschedulable[]` and `completion_journal_error`,
both written up in `lisa json-guide`.

**Reproduce it.** `a_completion_whose_commit_never_landed_is_finished_by_one_command`
in `crates/lisa-plugin/src/tests/rejected_has_an_exit.rs` drives the field
sequence against a real repository: completion goes in flight, its commit never
lands, the reconciliation deadline passes, it parks action-required — then one
call to `already-done` seals it. No file is edited by hand; the assertion that
`git status --porcelain docs/active/tickets` is empty is what enforces that.

## Tests

New:

- `a_completion_whose_commit_never_landed_is_finished_by_one_command` — the field sequence, end to end.
- `a_run_says_why_it_will_not_take_a_ready_ticket` — one line in the feed, and only one however many passes run.
- `a_rejection_whose_seal_never_landed_is_sealed_by_the_command` — the same case through the real binary.
- `a_reviewers_block_is_refused_and_lisas_own_recording_block_is_not`.
- `a_ticket_no_run_will_take_is_counted_blocked_and_named` — prose and JSON, together.
- `a_journal_that_cannot_be_read_is_reported_rather_than_fatal`.
- `the_guide_names_the_tickets_no_run_will_take`.

Extended: the two existing recovery tests carry `SealSource::Adopted` and now
also assert that the rejection at the bound names `already-done`.

`just check` (fmt + clippy `-D warnings` + `cargo test --workspace`) passes: 0
failures. `cargo build -p lisa-plugin --target wasm32-wasip1 --release` builds
clean at 2,252,007 bytes.

Beyond the suite, the field board was rebuilt by hand in a scratch project —
ticket at `phase: ready`, work committed, journal rejected action-required — and
walked through:

```
Status: 0 done, 0 in progress, 0 ready, 1 blocked

Tickets no run will take
  T-004-01     Lisa could not record its finished work, and nothing has settled that yet.
               To finish it: lisa already-done T-004-01

$ lisa already-done T-004-01
T-004-01 is finished — its work was here and the finishing record wasn't, so I wrote it: 70d29354.

Status: 1 done, 0 in progress, 0 ready, 0 blocked
```

## What still concerns me

**A ticket whose Done bytes are already committed cannot be sealed.** If the
ticket file at HEAD already reads `done` and the work directory is committed,
the transaction has nothing to commit and refuses with *"has no changes in the
requested include paths"*. `already-done` reports that error and changes nothing
— which is honest, and still a dead end. It is only reachable if someone edited
the ticket to Done by hand and committed it, which is the thing every command
here exists to stop them doing. Worth a follow-up (an allow-empty seal commit),
not worth widening this ticket.

**The journal `reason` grew a sentence.** Rejected rows now read
`<technical reason> — <ask>`. Nothing parses that field — it is carried as an
opaque `LaunchFailure` message — but any operator tooling grepping for the exact
old string will see a longer line.

**`lisa status` now folds the completion journal.** A fold failure is reported
as a line rather than propagated, deliberately: a journal that will not replay
is precisely when an operator types `lisa status`, and 0.4.4 already taught us
what fail-closed journal handling costs on a live board. The cost is that
`unschedulable` is silently short in that case, which is why the JSON carries
`completion_journal_error` beside it.

**A ticket in `implement`/`review` was double-counted before this change** —
`get_ready_tickets` and `get_in_progress_tickets` both claim it, so the counts
have summed past the total for a while. `75c44eb` stops this change making that
worse, but the underlying overlap in `DagStats` is untouched and out of scope.

**The mini is still on 0.5.0 from `lisa-nightly`.** Nothing here reaches that
machine until a release ships, and `T-070-01-01`'s sibling-deadlock fix is
waiting behind the same door.
