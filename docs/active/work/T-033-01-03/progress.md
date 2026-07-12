# Progress: gate ownership on acknowledgment

## Status

Implementation is complete and verified. The ticket-owned source change is ready for the
required isolated commit. Review remains after commit and post-commit audit.

## Baseline

The worktree was already dirty before this ticket began.

Preexisting modified paths included:

- `.lisa/hooks/on-clear.sh`;
- `.lisa/hooks/on-heartbeat.sh`;
- `.lisa/hooks/on-stop.sh`;
- `crates/lisa-cli/src/agent_exec.rs`;
- several active and knowledge documents.

Many story, ticket, work, PM, and runtime paths were also untracked. Those paths are not owned
by this ticket and were not included in its implementation boundary.

Prerequisite commits observed at baseline:

- `47e64b4 feat: model recycled Codex seat assignments`;
- `9cafcea feat: detect ticket-scoped Codex acknowledgments`.

Baseline focused tests passed:

```text
cargo test -p lisa-plugin codex_ack
  9 passed

cargo test -p lisa-plugin recycled_codex
  1 passed
```

## Phase artifacts completed

- `research.md`: mapped assignment state, adapter delivery, hook generation, signal polling,
  detector behavior, tests, and scope constraints.
- `design.md`: selected data-bearing pending state, monotonic generation, adapter tagging, raw
  payload transport, and exact-match promotion.
- `structure.md`: defined the file-level blueprint and verification boundary.
- `plan.md`: sequenced implementation, tests, isolated commit, and review.

No ticket frontmatter phase or status field was edited.

## Implementation unit 1: assignment identity

Completed in `crates/lisa-plugin/src/lib.rs`.

- Changed `AssignedPendingAck` to carry `generation: u64`.
- Added `State::next_assignment_generation`.
- Added a process-local nonzero generation allocator.
- Added `pending_assignment_generation` for exact state queries.
- Recycled/reused Codex scheduling now allocates one generation per delivery attempt.
- Fresh Codex remains immediately `Owned`.
- All Claude paths remain immediately `Owned`.
- Clear and exit timeout paths preserve the pending generation.

## Implementation unit 2: prompt identity

Completed in `crates/lisa-plugin/src/adapter.rs`.

- Extended `SpawnContext` with optional assignment generation.
- Added a single Codex assignment-prompt builder.
- Reuse prompts receive the canonical `LISA_ASSIGNMENT` marker while pending.
- Cross-provider shell launches receive the same marker.
- Structured JSON quotes are escaped in the shell representation so Codex receives valid marker
  JSON rather than shell-stripped keys and values.
- No-generation Codex prompts retain their prior content.
- Claude ignores the optional field and retains byte-equivalent adapter tests.

`crates/lisa-plugin/src/codex_ack.rs` now has live consumers, so its prerequisite-only
`allow(dead_code)` attributes were removed.

## Implementation unit 3: exact promotion

Completed in `crates/lisa-plugin/src/lib.rs`.

Added `acknowledge_codex_assignment(pane_id, payload_json) -> bool`.

The method:

- requires a current pane reservation;
- requires `AssignedPendingAck { generation }`;
- passes current ticket and generation to `detect_codex_ack`;
- writes `Owned` only on a detector match;
- returns true only for the actual pending-to-owned transition.

An already-owned seat has no pending identity, so a duplicate payload returns false. Released,
unknown, malformed, stale-ticket, and stale-generation payloads also return false.

## Implementation unit 4: lifecycle transport

Completed in `crates/lisa-cli/src/templates.rs` and `crates/lisa-cli/src/init.rs`.

- Added managed `on-ack.sh` template.
- The hook copies raw stdin JSON to a pane-scoped temporary file.
- It atomically renames the temporary file to `pane-<id>.ack`.
- Generated `.codex/hooks.json` now binds `UserPromptSubmit`.
- Merge logic adds exactly one Lisa ack hook while preserving user hooks.
- Init creates or safely updates the new managed script.
- Init makes the script executable when Lisa writes it.
- Validate requires the script and current Codex binding.
- Empty-project expected file count was updated from 20 to 21 planned creates.

## Implementation unit 5: scheduler signal consumption

Completed in `crates/lisa-plugin/src/lib.rs`.

- Added `check_codex_ack_signals`.
- It recognizes only `pane-<u32>.ack`.
- It reads payload content before deletion.
- It consumes validly named files regardless of classification result.
- It invokes the detector-gated promotion method.
- It bumps activity and logs only after a successful transition.
- It runs from `poll_tick` before transition timeout and future recovery evaluation.

## Implementation unit 6: documentation

Completed in `crates/lisa-cli/data/hooks-guide.md`.

- Updated lifecycle hook count from four to five executable scripts.
- Documented the ack payload file and its meaning.
- Distinguished timestamp signals from the raw JSON ack payload.
- Added `on-ack.sh` to scaffold inventory.
- Added `UserPromptSubmit` to manual Codex setup and validation guidance.

This file was added to the source ownership boundary as a plan deviation because leaving the
embedded product guide unchanged would make shipped setup documentation false.

## Acceptance test

`test_recycled_codex_ownership_requires_matching_ack_exactly_once` now exercises the complete
scheduler state sequence:

1. a recycled Codex seat enters generation-bearing pending state;
2. it reports not-owned without acknowledgment;
3. a previous-ticket payload is rejected;
4. a previous-generation payload is rejected;
5. exact current ticket and generation promote it;
6. the seat reports owned;
7. the same payload is rejected as a duplicate;
8. the seat remains owned.

`test_codex_ack_signal_promotes_matching_pending_seat` separately proves the file scanner removes
the ack file, promotes once, consumes a duplicate, and emits only one transition log.

## Adapter and hook tests

- `codex_pending_delivery_tags_launch_and_reuse_prompt` covers both prompt representations.
- Existing Codex no-generation and Claude adapter tests remain green.
- `test_on_ack_hook_preserves_payload_atomically` covers the shell template contract.
- `test_codex_hooks_json_contains_native_tui_signals` covers generated hook JSON.
- `test_merge_codex_hooks_preserves_user_hooks_and_is_idempotent` covers safe upgrades.
- Init round-trip validation covers hook/script creation and validation.

## Focused verification

Passed:

```text
cargo test -p lisa-plugin test_recycled_codex_ownership_requires_matching_ack_exactly_once
cargo test -p lisa-plugin ack_signal
cargo test -p lisa-plugin adapter
cargo test -p lisa-cli templates
cargo test -p lisa-cli init
cargo test -p lisa-cli hooks_guide
```

The first focused CLI run exposed one expected-count assertion after adding the sixth scaffolded
hook file (five executable plus notification sample). The expectation and comment were corrected;
the rerun passed.

## Package verification

Passed:

```text
cargo test -p lisa-plugin
  262 passed

cargo test -p lisa-cli
  269 unit tests passed
  1 atomic provider-contract integration test passed
```

## Workspace and target verification

Passed:

```text
cargo test --workspace
cargo fmt --all -- --check
cargo clippy -p lisa-plugin --lib -- -D warnings
cargo clippy -p lisa-cli --bin lisa -- -D warnings
just check
```

`just check` completed both:

- `cargo check -p lisa-plugin --target wasm32-wasip1`;
- `cargo test --workspace`.

## Clippy baseline exception

The broader command below was attempted:

```text
cargo clippy --workspace --all-targets -- -D warnings
```

It is blocked by preexisting test-only lints outside this ticket's changes:

- twelve `clippy::unnecessary_to_owned` findings in `crates/lisa-core/src/dag.rs`;
- one `clippy::needless_borrows_for_generic_args` finding in an existing
  `crates/lisa-cli/src/init.rs` test around line 2045.

Production-target strict Clippy passes for both modified crates. The unrelated baseline was not
changed or included in this ticket.

## Diff verification

Passed `git diff --check` on all ticket-owned source paths.

Source ownership now includes exactly:

- `crates/lisa-plugin/src/lib.rs`;
- `crates/lisa-plugin/src/adapter.rs`;
- `crates/lisa-plugin/src/codex_ack.rs`;
- `crates/lisa-cli/src/templates.rs`;
- `crates/lisa-cli/src/init.rs`;
- `crates/lisa-cli/data/hooks-guide.md`.

The ticket file remains untracked as supplied by Lisa and has no diff. Work artifacts remain
untracked for Lisa's completion transaction.

## Deviations from plan

1. Added the embedded hooks guide to the source change inventory after diff review showed the
   shipped documentation still claimed only four scripts and three Codex bindings.
2. The all-target Clippy gate could not pass because of unrelated existing test lints. Equivalent
   strict production-target Clippy checks passed.
3. No separate rejected-file scanner test was added because the primary scheduler test covers
   stale payload rejection and the scanner test covers duplicate-file consumption; the detector
   fixture suite already covers malformed and stale payload classification.

## Isolated source commit

The globally installed Lisa binary did not recognize `commit-ticket`. The repository-built CLI
was used instead:

```text
cargo run -p lisa-cli -- commit-ticket \
  --ticket-id T-033-01-03 \
  --message "feat: gate Codex seat ownership on acknowledgment" \
  --include <six exact owned paths>
```

Result:

```text
74afa61fe81fd95c68e21919dd7d57c01a7063a4
```

Commit summary:

```text
74afa61 feat: gate Codex seat ownership on acknowledgment
6 files changed, 360 insertions(+), 54 deletions(-)
```

The six committed paths exactly match the ownership list above. No work artifact or ticket file
was included.

## Post-commit audit

- All six ticket-owned source/documentation paths are clean.
- No ticket-owned source path is staged, modified, or untracked.
- The ordinary index has no staged paths.
- Ticket and work artifacts remain untracked for Lisa's completion transaction.
- The ticket frontmatter is still `status: open`.
- Lisa automatically advanced its phase after detecting artifacts; the agent did not edit phase
  or status fields.
- Unrelated modified and untracked paths remain present and were excluded from the commit.

## Remaining before Review

- write `review.md`;
- stop without editing ticket phase/status or publishing completion.
