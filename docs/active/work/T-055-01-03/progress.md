# Progress — T-055-01-03 · a-way-out-of-rejected

Seven steps from plan.md. Each row updates when the step's commit lands.

| # | Step | State |
|---|---|---|
| 1 | Completion-key trailer helpers in `lisa-core::completion` | done — `6dc406e` |
| 2 | Move journal record/fold/append into `lisa-core` | done — `ada0d3a` |
| 3 | Thread `DispositionOrigin` through (no behavior change) | done — `c2fd64b` |
| 4 | Recording failure stops reading as a verdict (AC3) | done — `149c0c6` |
| 5 | Bound the re-attempt (AC2) | done — `0c3ad80` |
| 6 | `lisa already-done` + unblock's decline (AC1, AC4, AC5) | done — `432f3f9` |
| 7 | Field-sequence test + `just check` (AC1 test clause, AC6) | done — `a35e643` |
| 8 | README entry for the new command (unplanned) | done — `85d8283` |
| 9 | Resume an interrupted recovery (unplanned, self-review) | done — `b74228c` |

## Log

- Started Implement. No deviations from plan.md yet.
- Step 1: `COMPLETION_KEY_PREFIX`, `completion_key_marker`,
  `completion_key_ticket_prefix` added beside `CompletionGenerationId`; `write_hex`
  refactored onto a shared `hex()` with `Display` output unchanged (pinned by the
  existing generation-key test).
- Step 2: 12 fold/append tests moved to `lisa-core` unedited; 4 seal tests stayed in
  the plugin. `append_with_seal` became `append_with_seal_using(..., publish)` in core,
  with a one-line plugin wrapper supplying `RustPublication` — every plugin call site
  unchanged. Four accessors that were `#[cfg(test)]` in the plugin
  (`seal`, `failure_limit`, `confirmed_commit_id`, `confirmed_receipt`) are now part of
  the shared surface, since the recovery command reads them in production.
  New test `append_publishes_only_after_the_whole_history_folds` pins the anti-brick
  property. `just check` exit 0.
- Step 3: `DispositionOrigin` on `Block` and `ParkedRemedy`, tolerant parsing of an
  optional `"origin"` key with an unreadable value failing toward `Review`. The strict
  authoring check is untouched and its extra-field rule already refuses an
  agent-authored `origin` — pinned by a new test rather than by new code.
  `crates/lisa-cli/src/status.rs` joined the include set: it builds `ParkedRemedy`
  fixtures and would not compile otherwise.
- Step 4: `completion_failure_ask` now returns `String` rather than `Option<String>` —
  the `None` for `Unrecognized` is what let a command's stderr stand in as the ask and,
  through the park, as the review reason. Three existing plugin tests asserted the old
  conflation and were rewritten to assert the separation, each also asserting the raw
  text is still in the journal.
- Step 5: bound is `MAX_ACTION_REQUIRED_GENERATIONS = 2`, read from the journal so it
  survives restart. Guards both coercions (`reconciliation_state`'s open-status arm and
  `dispatch_completion`'s operator arm), the `[d]` refusal, `send_back_for_review`, and
  the exhausted ask. **Deviation from plan.md step 5:** the planned unpark test drove
  generation 2 through `Reconcile`, which cannot re-arm after a park because the
  canonical disposition is then a block and `reconcile` refuses a non-passing verdict.
  Rewritten as a differential — the same unpark re-arms at one action-required
  generation and does not at two — which pins the bound rather than the disposition
  gate. Added an unplanned `send_back_declines_past_the_bound_and_points_at_the_command`
  for the `[s]` half of "no seat, no pane".
- Step 6: `lisa already-done <TICKET_ID>` as a visible operator command (display order
  renumbered: unblock 4, already-done 5, doctor 6, proposal 7, loop 8).
  `MAX_ACTION_REQUIRED_GENERATIONS` moved from the plugin to
  `lisa_core::completion_journal` — **deviation from structure.md**, which put it in
  `lib.rs`; `lisa unblock` needs the same number, and two crates disagreeing about
  where the bound is would be a real bug. Two unplanned files joined the include set:
  `crates/lisa-cli/tests/help_surface.rs` (17→18 commands, snapshots, test renamed) and
  `docs/knowledge/flag-audit.md`, whose live-CLI audit test fails on any unlisted flag.
- Step 7: `rejected_has_an_exit.rs` drives the whole field sequence against a real
  repository — in-flight generation, real `complete_ticket` commit (asserting it writes
  no journal row, AC4's plumbing clause), deadline expiry to action-required, an
  operator signature that fails on the empty-diff error and reaches the bound, a third
  press that launches nothing, then recovery. Asserts the journal ends `Confirmed` with
  the original commit id, the board reads Done, masking has stopped, a simulated plugin
  restart reads the same, and a second run declines.
- Step 8 (unplanned): the ticket asks for a **documented** operator route, so the README
  command reference and the completion-seal section name `lisa already-done`.

- Step 9 (unplanned, found in self-review): the command writes three journal records,
  so a killed process left a half-written recovery generation reading `Requested` —
  which masks the board's Done and would have been refused as "still working on it".
  A fresh dead end inside the command built to remove dead ends. `already-done` now
  resumes an operator generation it finds part-written, and a test drives both
  interruption points.

## Result

`just check` exit 0 after every step. Ten commits, no ticket-owned file left staged,
modified, or untracked. `crates/lisa-cli/src/commit_transaction.rs` — T-055-01-02's
file — was never edited; that ticket completed in `f15bf1b`, before this ticket's
first commit, so its convergence fix is in this baseline.
