# Plan — T-055-01-03 · a-way-out-of-rejected

Seven steps. Each ends green (`cargo test --workspace` at minimum) and is one
`lisa commit-ticket` unit. Steps 1–3 change no behavior; 4–7 each close a criterion.

---

## Step 1 — the completion-key trailer moves next to the key

**Change.** `crates/lisa-core/src/completion.rs`: add `COMPLETION_KEY_PREFIX`,
`completion_key_marker`, `completion_key_ticket_prefix`; refactor the private `write_hex`
formatter onto a shared `hex(&[u8]) -> String` with `Display` output unchanged.

**Tests.** In-module: the prefix literal is exactly `"Lisa-Completion-Key: "`; the marker for a
known key equals `"Lisa-Completion-Key: "` + that key's `Display`; the ticket prefix is a
proper prefix of the marker for generations 1 and 7 and for two different attempt ids; and
`CompletionGenerationId::to_string()` still returns the value already pinned by
`completion_generation_binds_ticket_attempt_and_generation`.

**Verify.** `cargo test -p lisa-core`.

**Commit.** `--include crates/lisa-core/src/completion.rs`

---

## Step 2 — move the journal's record/fold/append half into lisa-core

Largest and most mechanical step. Nothing about the format, the records, or the reducer
changes; only where the code lives and how it publishes.

**Change.**
- New `crates/lisa-core/src/completion_journal.rs`: lines 1–472 and 677–1058 of the plugin
  module, visibility widened per structure.md §A2, `append_with_seal` replaced by
  `append_with_seal_using(path, seal, transition, publish)`, plus
  `COMPLETION_JOURNAL_RELATIVE_PATH`.
- `crates/lisa-core/src/lib.rs`: `pub mod completion_journal;`.
- `crates/lisa-plugin/src/completion_journal.rs`: keeps the seal half; adds the `pub(crate) use`
  re-exports and the `append_with_seal` wrapper that supplies `RustPublication`.
- `crates/lisa-plugin/src/lib.rs`: import path updates only; `.lisa/completion-journal.jsonl`
  (~9298) reads the new constant.

**Tests.** Move — not rewrite —
`requested_in_flight_and_confirmed_reconstruct_after_each_restart`,
`failed_command_observations_are_bounded_durable_and_restart_safe`,
`failure_observation_rejects_skips_limit_changes_and_overrun`,
`retryable_rejection_can_start_another_request_generation`,
`a_new_attempt_key_can_start_after_a_confirmed_ticket_is_reset`,
`torn_malformed_empty_and_unknown_records_fail_closed`,
`invalid_key_correlation_and_order_leave_prior_bytes_unchanged`,
`legacy_in_flight_without_deadline_loads_expired_and_action_required_masks_done`,
`new_rows_carry_the_pinned_seal_and_mixed_generations_fail_closed`, with their `key`,
`correlation`, `deadline` helpers. The seal tests stay in the plugin. One new core test —
`append_publishes_only_after_the_whole_history_folds` — asserts a journal with an unreplayable
line makes `append_with_seal_using` return `Err` **without invoking the publish closure**. That
is the property the whole design rests on and it was implicit before.

**Verification criterion.** Test count across the workspace is unchanged apart from the one
addition, and no moved test's body is edited.

**Verify.** `cargo test --workspace`, then
`cargo build -p lisa-plugin --target wasm32-wasip1 --release`.

**Commit.** `--include crates/lisa-core/src/completion_journal.rs crates/lisa-core/src/lib.rs crates/lisa-plugin/src/completion_journal.rs crates/lisa-plugin/src/lib.rs`

---

## Step 3 — thread `DispositionOrigin` through, behavior unchanged

**Change.** `disposition.rs` (the enum, the `Block` field, tolerant parsing of `"origin"`),
`parking.rs` (`ParkedRemedy.origin`), and the mechanical pattern updates in
`lisa-plugin/src/lib.rs`, `lisa-cli/src/check_disposition.rs`, `lisa-cli/src/proposal.rs`,
`lisa-core/tests/completion_state_machine.rs`.

**Tests.** In `disposition.rs`: absent `origin` parses as `Review`; `"internal-command"` parses
as `InternalCommand`; a junk `origin` value degrades to the unstructured operator fallback with
`origin: Review` (fail-safe direction); `check_review_disposition` still rejects a document
carrying `origin` with its existing "remove extra block fields" fix. In `parking.rs`: the
projection carries origin through.

**Verify.** `cargo test --workspace`.

**Commit.** the six files above.

---

## Step 4 — a recording failure stops reading as a verdict *(closes AC3)*

**Change.** `lisa-plugin/src/lib.rs`: `completion_failure_ask` gains the `exhausted` input and
a non-`None` answer for `Unrecognized`; `park_failed_completion` writes
`origin: "internal-command"` and a `reason` that states the boundary rather than quoting the
command.

**Tests.** `operator_recovery_matrix.rs::a_recording_failure_is_not_a_reviewers_block`:
force a completion failure whose stderr is the field string
(`ticket T-x has no changes in the requested include paths`), then parse the published
canonical disposition and assert
(a) `origin == DispositionOrigin::InternalCommand`,
(b) `reason` does **not** contain the stderr text and does not contain `"Error:"`,
(c) a reviewer-authored block from the same fixture parses with `origin == Review`, so the two
are separable without reading prose,
(d) the raw text is still present in the journal's rejection row — the detail is moved, not
lost.

**Verify.** `cargo test -p lisa-plugin`.

**Commit.** `--include crates/lisa-plugin/src/lib.rs crates/lisa-plugin/src/tests/operator_recovery_matrix.rs`

---

## Step 5 — the re-attempt is bounded *(closes AC2)*

**Change.**
- `lisa-core/src/completion_journal.rs`: `action_required_generations` on the aggregate,
  maintained in `apply_transition`'s `Rejected` (increment) and `Requested` (carry forward)
  arms.
- `lisa-plugin/src/lib.rs`: `MAX_ACTION_REQUIRED_GENERATIONS`,
  `recovery_generations_exhausted`, both coercions guarded, `send_back_for_review` declining,
  and the exhausted ask naming `lisa already-done`.

**Tests.**
- Core: `action_required_generations_survive_a_new_key_and_stop_at_the_bound` — fold a journal
  through two rejected generations and assert the counter is 2 after the second `Requested`
  reset, and that a `Confirmed` generation does not increment it.
- Plugin `repeated_done_key_stops_at_the_bound_and_names_the_command`: press `[d]`, fail,
  press again, fail, press a third time — assert exactly two effects launched, the third
  produces a `CompletionRejected` event, the ticket is `blocked`, the agent slot's `ticket_id`
  is `None` and `threads` has no entry (**no seat, no pane**), and the canonical disposition's
  `ask` contains `lisa already-done T-…`.
- Plugin: after `lisa unblock`'s flip (status `open`) past the bound, a reconcile pass produces
  no effect — the "re-attempts on every loop start" regression.
- `parking::validate_block_ask` accepts the new exhausted ask (guards the ≤160-char,
  action-verb lead).

**Verify.** `cargo test --workspace`.

**Commit.** `--include crates/lisa-core/src/completion_journal.rs crates/lisa-plugin/src/lib.rs crates/lisa-plugin/src/tests/operator_recovery_matrix.rs`

---

## Step 6 — `lisa already-done` *(closes AC1 and AC4)*

**Change.** New `crates/lisa-cli/src/already_done.rs`; `lisa-cli/src/lib.rs` test-support
export; `main.rs` subcommand, display-order renumber and dispatch; `unblock.rs`'s exhausted
decline; `help_surface.rs` updated to 18/9 commands with the new snapshots.

**Tests.**
- `crates/lisa-cli/tests/already_done.rs` (black-box against the built binary, matching the
  crate's convention):
  - **the negative fixture** — a `Rejected{ActionRequired}` journal with **no** keyed commit in
    history: exits non-zero, names that it cannot find the work in history, and leaves the
    journal bytes and the ticket file byte-identical. A "take the operator's word" implementation
    fails here.
  - an unknown ticket, a `Confirmed` aggregate, a ticket with no aggregate, and a
    journal-sealed project each decline with their own message and change nothing.
  - a torn journal makes the command fail without writing (the anti-brick property).
- `crates/lisa-cli/tests/parked_ux.rs` — every existing case unchanged (**AC5**), plus one new
  case: `lisa unblock` on an exhausted ticket declines, names `lisa already-done`, and leaves
  `status: blocked`.
- `help_surface.rs` — the new command resolves, appears in the operator listing, carries an
  example, and its help text trips no banned jargon.

**Verify.** `cargo test --workspace`.

**Commit.** `--include crates/lisa-cli/src/already_done.rs crates/lisa-cli/src/lib.rs crates/lisa-cli/src/main.rs crates/lisa-cli/src/unblock.rs crates/lisa-cli/tests/already_done.rs crates/lisa-cli/tests/help_surface.rs crates/lisa-cli/tests/parked_ux.rs`

---

## Step 7 — the field sequence, end to end *(closes AC1's test clause)*

**Change.** New `crates/lisa-plugin/src/tests/rejected_has_an_exit.rs`, registered in the
plugin's test module list.

**The test.** One function, driven against a real temporary git repository:

1. Build the review fixture and seal generation 1 through
   `lisa_cli::commit_transaction::complete_ticket` — a real commit carrying
   `Lisa-Completion-Key` for `(ticket, "1", 1)`. Journal it `Confirmed`.
2. Reset the board the way a lost race leaves it: a generation-2 operator completion whose
   command returns the empty-include-path error. Drive it through
   `handle_completion_result`, so the real classifier, the real budget and the real
   `park_failed_completion` run. Assert the journal now folds to
   `Rejected { retryability: ActionRequired }` and the board reads `blocked` at Review while
   the work is committed at HEAD — the exact field state.
3. Assert the pre-fix dead end still holds at its own boundary: `lisa unblock`'s flip returns
   the ticket to `open`, and a reconcile pass past the bound produces no effect.
4. Run `lisa_cli::already_done::run_already_done`.
5. Assert:
   - it returns `Recovered` with generation 1's commit id;
   - the journal folds to `Confirmed` with a `Commit` receipt carrying that id;
   - `masks_durable_done()` is false, so the board is no longer masked;
   - the ticket file reads `status: done` / `phase: done`;
   - a plugin restart (`load` into a fresh `State`) still reads `Confirmed` — the record is
     durable, not in-memory.
6. **AC4's plumbing clause:** in the same repository, a direct
   `lisa_cli::commit_transaction::complete_ticket` call still succeeds and still writes no
   journal row, proving the plumbing is unchanged while the supported route is the one that
   agrees with the journal.

**Verify.** `just check` — fmt, clippy, WASM check, workspace tests. This is the criterion's
own gate, and per the standing rule it is judged by exit code, never by grepping output.

**Commit.** `--include crates/lisa-plugin/src/tests/rejected_has_an_exit.rs crates/lisa-plugin/src/lib.rs`

---

## Testing strategy, by criterion

| AC | Where it is proven | Kind |
|---|---|---|
| 1 — one documented command moves rejected → terminal, journal + board agree | `rejected_has_an_exit.rs` steps 4–5 | integration, real git, real journal |
| 1 — the command exists and is documented | `help_surface.rs` | black-box CLI |
| 2 — re-attempts bounded, named state, named command | `operator_recovery_matrix.rs`, core counter test | unit + adapter |
| 3 — transport failure distinguishable by field | `operator_recovery_matrix.rs`, `disposition.rs` | unit |
| 4 — recovery writes a journal transition; plumbing still plumbing | `rejected_has_an_exit.rs` steps 5–6 | integration |
| 5 — unblock unchanged for cases that work | `parked_ux.rs` (unedited existing cases) | black-box CLI |
| 6 — `just check` green | step 7 | gate |

## Deviations

Recorded in `progress.md` as they happen, with the reason, before proceeding.

## Risks and their guards

- **The move (step 2) is the largest diff.** Guard: it is behavior-preserving, tests move
  unedited, and it lands as its own commit with the WASM build verified before anything rides
  on it.
- **Bound value.** `MAX_ACTION_REQUIRED_GENERATIONS = 2` is a judgement. Guard: it is a named
  constant next to `MAX_COMPLETION_FAILURES`, and the tests assert *the bound holds*, not the
  specific number, except in one place that names it.
- **Trailer drift** between the new reader and the transaction's private
  `discover_completion_commit`. Guard: step 7's test finds a commit created by the real
  `complete_ticket` using the new reader — drift fails the test.
- **`lisa-cli/src/commit_transaction.rs` is owned by T-055-01-02.** Guard: it is absent from
  every `--include` list above.
