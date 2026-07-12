# T-033-03-02 Progress — consecutive reuse live proof

## Status

Implementation complete. Research, Design, Structure, Plan, and Implement are
complete. The native repeated-lifecycle regression, harness, generated report,
broad verification, and isolated source commit are complete. Review remains.

## Completed

### Repository and contract mapping

- Read `AGENTS.md`, `CLAUDE.md`, the ticket, and the RDSPI workflow.
- Mapped the dependency regression, acknowledged-assignment state machine,
  release/reuse path, adapter reset strategies, harness precedent, and dirty
  worktree boundaries.
- Wrote `research.md`.

### Design and implementation blueprint

- Selected a production-state native regression plus a validating report
  runner.
- Defined a two-pane, ten-Codex-assignment proof and equivalent ten-assignment
  Claude control.
- Wrote `design.md`, `structure.md`, and `plan.md`.

### Native scenario

- Added a test-only multi-ticket fixture builder in
  `crates/lisa-plugin/src/lib.rs`.
- Added test-only helpers to refresh the fixture DAG, collect active assignments,
  and submit an exact matching Codex acknowledgment.
- Added
  `test_consecutive_reused_panes_resolve_codex_ack_or_fallback_and_preserve_claude`.
- The Codex scenario drives panes 10 and 11 through five rounds / ten unique
  resident-session reassignments.
- Assignments 1–5 and 7–10 resolve `ack-then-owned`.
- Assignment 6 intentionally omits the original acknowledgment, evaluates its
  exact deadline, observes `Recovering`, records one fresh launch, acknowledges
  the recovery generation, and resolves `timeout-then-fallback`.
- The Claude control drives panes 20 and 21 through the same five rounds / ten
  unique reassignments, preserving `WaitingForClear -> Idle` transport and
  immediate `Owned` assignment state.
- The test emits 20 assignment evidence records and one summary record.

### Focused result

Passed:

```text
cargo test -p lisa-plugin consecutive_reused_panes \
  -- --nocapture --test-threads=1

1 passed; 0 failed
Codex rows: 10
ack-then-owned: 9
timeout-then-fallback: 1
Claude rows: 10
silent stalls: 0
```

Observed Codex pane set: `{10, 11}`.

Observed Claude pane set: `{20, 21}`.

### Harness and report

- Added executable `harness/run.sh`.
- Added `harness/README.md` with the exact proof boundary and evidence schema.
- Ran the harness successfully from the repository root and from `/tmp`.
- Generated `run-report.md` from an actual passing harness execution.
- The runner independently validates 21 evidence records: 20 assignments plus
  one exact summary.
- `bash -n` and ShellCheck both pass.

### Focused neighboring regressions

Passed:

```text
cargo test -p lisa-plugin dropped_post_prompt_ack
cargo test -p lisa-plugin bounded_ack_wait
cargo test -p lisa-plugin reused_claude_assignment
```

Each command ran one selected test with zero failures.

### Package and workspace verification

Passed:

```text
cargo test -p lisa-plugin
267 passed; 0 failed
```

Passed:

```text
cargo test --workspace
lisa-cli unit tests: 270 passed
atomic provider integration: 1 passed
lisa-core unit tests: 150 passed
lisa-plugin unit tests: 267 passed
doc tests: 0 failed
total executed tests: 688 passed; 0 failed
```

### Quality and target verification

Passed:

```text
cargo fmt --all -- --check
cargo clippy -p lisa-plugin --all-targets -- -D warnings
cargo check -p lisa-plugin --target wasm32-wasip1
bash -n docs/active/work/T-033-03-02/harness/run.sh
shellcheck docs/active/work/T-033-03-02/harness/run.sh
git diff --check -- <ticket-owned paths>
```

### Isolated source commit

The installed `/opt/homebrew/bin/lisa` is older and returned `unrecognized
subcommand 'commit-ticket'`. Per Plan, the just-built workspace CLI was used:

```text
target/debug/lisa commit-ticket \
  --ticket-id T-033-03-02 \
  --message "test: prove consecutive acknowledged Codex reuse" \
  --include crates/lisa-plugin/src/lib.rs \
  --include docs/active/work/T-033-03-02/harness/README.md \
  --include docs/active/work/T-033-03-02/harness/run.sh \
  --include docs/active/work/T-033-03-02/run-report.md
```

Commit:

```text
66fc5fcfc654b91fe040ce7bd61dbefa76295522
```

The commit contains exactly the four intended implementation/evidence paths.
`crates/lisa-plugin/src/lib.rs` is clean afterward. The ordinary index is empty.
Only the ticket and five phase artifacts remain untracked for Lisa's completion
transaction; `review.md` is the final remaining artifact.

## Deviations from plan

### Corrected Codex transport assumption

The initial Research/Design draft described native Codex reuse as the adapter's
`FreshExec` reset strategy. The first focused run disproved that assumption:
the current `CodexAdapter` uses the same `ClearHandshake` transport as Claude.

Observed state immediately after scheduling was correctly:

```text
transition = WaitingForClear
assignment = AssignedPendingAck { deadline: None }
```

The scenario now delivers `handle_cleared_signal` before reading the armed
deadline. Research, Design, Structure, and Plan were corrected immediately.
The provider distinction under test is ownership semantics after the shared
transport: Codex remains pending until ticket/generation ack; Claude is already
owned.

No production change was needed.

### Explicit fixture completion phase

The scheduler updates Ready to Research on launch, but the consecutive fixture
needs each consumed ticket excluded deterministically before rebuilding its
DAG. The test explicitly updates each temporary ticket to `Phase::Done` after
marking its thread completed and before release/recompute.

This models the production completion boundary and avoids depending on artifact
polling inside a lifecycle-focused unit test. It touches only temporary files.

## Remaining

- Write `review.md`, then stop without changing ticket phase or status.
