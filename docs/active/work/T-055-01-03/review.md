# Review — T-055-01-03 · a-way-out-of-rejected

Ten commits, `just check` exit 0 after each. 1368 workspace tests pass.

## What the operator can now do that they could not

Run `lisa already-done <ticket>`. If a commit reachable from HEAD carries that
ticket's `Lisa-Completion-Key`, Lisa marks the ticket done and writes a terminal
journal record, so board and journal agree. If no such commit exists, nothing
changes and Lisa says so — the operator's word alone is never enough.

And Lisa stops asking to be rescued forever: after two generations end
action-required, it stops re-attempting, stays parked, gives up its seat and pane,
and names `lisa already-done` in the ask, the `[d]` refusal, `[s]`'s decline, and
`lisa unblock`'s decline.

## Files

**New**

| File | What |
|---|---|
| `crates/lisa-core/src/completion_journal.rs` | The journal's records, fold, and append, moved out of the plugin so two crates can write it through one validated path. |
| `crates/lisa-cli/src/already_done.rs` | The recovery command. |
| `crates/lisa-plugin/src/tests/rejected_has_an_exit.rs` | The field sequence, end to end, against a real repository. |
| `crates/lisa-cli/tests/already_done.rs` | The command's refusals, including the negative fixture. |

**Modified**

| File | What |
|---|---|
| `crates/lisa-core/src/completion.rs` | `COMPLETION_KEY_PREFIX`, `completion_key_marker`, `completion_key_ticket_prefix` beside the key they render. `Display` output unchanged. |
| `crates/lisa-core/src/disposition.rs` | `DispositionOrigin`; `Block` gained `origin`; tolerant parsing of `"origin"`. The strict authoring check is untouched. |
| `crates/lisa-core/src/parking.rs` | `ParkedRemedy.origin`. |
| `crates/lisa-plugin/src/completion_journal.rs` | Shrank to the journal *seal* half plus re-exports and a one-line `append_with_seal` wrapper. |
| `crates/lisa-plugin/src/lib.rs` | The bound, its two coercion guards, the `[d]` and `[s]` declines, the honest park disposition. |
| `crates/lisa-cli/src/unblock.rs` | One decline, for exhausted tickets only. |
| `crates/lisa-cli/src/main.rs`, `lib.rs` | The subcommand, display-order renumber, test-support export. |
| `crates/lisa-cli/src/status.rs`, `check_disposition.rs`, `proposal.rs`, `lisa-core/tests/completion_state_machine.rs` | Mechanical: the new field. |
| `README.md`, `docs/knowledge/flag-audit.md`, `crates/lisa-cli/tests/help_surface.rs`, `parked_ux.rs` | Documentation and its enforcement. |

**Untouched on purpose:** `crates/lisa-cli/src/commit_transaction.rs`, owned by
T-055-01-02. That ticket completed in `f15bf1b`, before this ticket's first commit,
so its convergence fix is in this baseline and nothing here conflicts with it.

## Acceptance criteria

| Criterion | Where it is proven |
|---|---|
| Rejected → terminal by one documented command; a test drives the field sequence and asserts journal terminal + board agrees | `rejected_has_an_exit::the_lost_race_that_could_not_be_recovered_now_can_be` |
| `MarkDoneKey` re-attempts bounded; test names the state and the command | `operator_recovery_matrix::repeated_done_key_stops_at_the_bound_and_names_the_command`, `an_unpark_past_the_bound_does_not_re_arm_the_completion`, `completion_journal::action_required_generations_survive_a_new_key_and_a_retryable_one_does_not_count` |
| Transport failure distinguishable by field; reason carries no command error | `operator_recovery_matrix::a_recording_failure_is_not_a_reviewers_block`, `disposition::a_recording_failure_and_a_reviewers_verdict_are_separable_by_field` |
| Recovery writes a journal transition; plumbing still plumbing | `rejected_has_an_exit` steps 2 and 7 — the direct `complete_ticket` call asserts the journal is byte-identical afterward |
| `lisa unblock` unchanged for cases that work today | Every pre-existing `parked_ux` case is unedited; `unblock_steps_aside_only_for_a_completion_lisa_stopped_recording` is a differential — one ticket at the bound declines, one under it reopens normally |
| `just check` green | Exit code 0, verified after every commit |

## How each behavior works, and why that way

**The recovery is a new generation, not a confirmation of the rejected one.**
`reduce()` refuses `CommandSucceeded` from `Rejected`, and a hand-appended row that
ignores that has bricked a real board (starfox, 2026-07-19: the fail-closed load
errors and *all* scheduling stops with one line in the feed). So `already-done`
appends `Requested → CommandInFlight → Confirmed` under a fresh operator generation,
which the format already permits. No new record type, no schema bump.

**Every append re-folds first.** That is the reason the journal's write half moved
into `lisa-core` rather than being reimplemented CLI-side: a second writer built on a
projection could not validate what it writes. `append_publishes_only_after_the_whole_history_folds`
pins that an unreplayable journal never reaches the publish closure, and
`an_unreplayable_journal_fails_the_command_instead_of_growing` pins the same at the
command's own boundary.

**Evidence is the key, never the emptiness.** `find_sealed_commit` greps for
`Lisa-Completion-Key: v1:<hex ticket>:` and re-reads each candidate's full message
before accepting it. Any generation of the ticket counts, because the rejected
generation is precisely the one that never committed. The trailer text has one
definition in `lisa-core`; the field-sequence test proves the new reader finds a
commit made by the real `complete_ticket`, so drift between it and the transaction's
own private discovery fails a test.

**The bound is durable.** `action_required_generations` is folded from records already
on disk and carried across the `Requested` reset that starts a new generation — which
is what `failure_count` deliberately does not do, and why re-attempting was bounded
per attempt and unbounded across loop starts. Old journals fold to the same number.

**Origin is a field, not a phrase.** `park_failed_completion` writes
`"origin":"internal-command"` and a reason that states which boundary failed; the
command's own text stays in the journal's rejection row and the activity feed, both
unchanged. `completion_failure_ask` returning `Option` was the actual mechanism of the
conflation — the `None` for `Unrecognized` is how stderr became the ask and then the
reason — so it returns `String` now and every class answers.

## Test coverage, and the gaps

Added: 2 core journal tests, 3 core disposition tests, 1 core parking test, 1 core
completion test, 3 plugin operator-recovery tests, 2 plugin field-sequence tests, 4 CLI
`already-done` tests, 1 CLI `parked_ux` test. Twelve journal fold tests moved to
`lisa-core` with their bodies unedited. Three pre-existing plugin tests were rewritten
because they asserted the conflation this ticket removes; each now also asserts the raw
command text is still in the journal, so the change is a move rather than a loss.

Honest gaps:

- **No test drives two genuinely concurrent completions into this recovery.** The field
  sequence reproduces the *state* a lost race leaves (in-flight generation, commit at
  HEAD, deadline expiry to action-required) rather than racing two processes.
  T-055-01-01's `guard_waits_its_turn.rs` owns the concurrency proof; composing them
  would be a slower test that proves the same two facts separately proven.
- **`MAX_ACTION_REQUIRED_GENERATIONS = 2` is a judgement, not a derivation.** The tests
  assert the bound holds and that behavior differs on either side of it; only one
  assertion names the number.
- **The recovery command does not commit the ticket file it rewrites.** It reports that
  the file changed and leaves the commit to the operator. Committing here would need the
  isolated transaction from a file another ticket owns, and would risk clobbering
  concurrent work.
- **No test for a repository where `git` itself is missing or broken mid-command.**
  `find_sealed_commit` returns a named `Err` in that case and nothing is written, but
  that path is unexercised.

## Open concerns for a human reader

1. **The journal move is the largest part of this diff** (~1500 lines out of the
   plugin, ~1667 into core) and is the piece most worth a skim. It is mechanical: the
   only behavioral change inside it is `action_required_generations`, and the only API
   change is `append_with_seal` becoming `append_with_seal_using(..., publish)`. Test
   bodies moved unedited so the move is checkable by reading names.

2. **Four accessors became part of the shared surface.** `seal`, `failure_limit`,
   `confirmed_commit_id` and `confirmed_receipt` were `#[cfg(test)]` in the plugin; the
   recovery command reads them in production, so they are now plain `pub`.

3. **`already-done` accepts a keyed commit from any generation.** That is deliberate
   and argued above, but it does mean the command settles a ticket on evidence that
   *some* completion sealed, not that *this* completion did. A ticket whose work was
   committed and then reverted would still be settled. Detecting that would mean
   diffing the commit against the working tree, which is a different and weaker test
   than the key.

4. **The bound changes what `[d]` does on a very stuck ticket.** Past two
   action-required generations, even a signed operator override is refused. That is
   correct — signing accepts the *work*, and the failure is not about the work — but it
   is a behavior change to a key an operator may have muscle memory for. The refusal
   names `already-done`, so it is a redirect rather than a wall.
