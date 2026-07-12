# T-033-03-01 Progress — deterministic stall reproduction

## Status

Implementation, verification, and isolated source commit complete.

## Completed workflow phases

- Research completed in `research.md`.
- Design completed in `design.md`.
- Structure completed in `structure.md`.
- Plan completed in `plan.md`.

## Source ownership

Planned ticket-owned source path:

```text
crates/lisa-plugin/src/lib.rs
```

The ticket file and work artifacts are reserved for Lisa's final completion
transaction. Existing unrelated modified and untracked paths will be preserved.
The ordinary Git index will not be used for ticket work.

## Implementation checklist

- [x] Confirm source baseline and dependency test.
- [x] Add explicit dropped post-prompt acknowledgment regression.
- [x] Prove historical open-loop false ownership facts.
- [x] Prove current pending state is unowned and deadline-bound.
- [x] Prove exactly one fresh recovery launch.
- [x] Prove absent recovery acknowledgment terminates actionably.
- [x] Run focused tests.
- [x] Run package and workspace tests.
- [x] Run formatting, lint, WASM, and diff checks.
- [x] Commit exact source path through Lisa's isolated transaction.
- [x] Confirm ticket-owned source cleanliness.

## Deviations

The clean dependency baseline contained one rustfmt drift in
`start_assignment_ack_wait`. The first `cargo fmt --all -- --check` reported
that pre-existing layout plus formatting needed in the new test. After manually
formatting the test, `cargo fmt -p lisa-plugin` normalized the dependency line
without changing behavior. This one formatting-only production hunk is retained
so the repository-wide formatting gate passes; scheduler logic and interfaces
remain unchanged.

## Verification log

### Baseline

`crates/lisa-plugin/src/lib.rs` had no worktree diff before implementation.
The ordinary index contained no listed entries. Existing unrelated modified and
untracked paths were left untouched.

Passed before editing:

```text
cargo test -p lisa-plugin bounded_ack_wait
1 passed; 0 failed
```

### Focused regression and neighbors

Passed:

```text
cargo test -p lisa-plugin dropped_post_prompt_ack
1 passed; 0 failed

cargo test -p lisa-plugin bounded_ack_wait
1 passed; 0 failed

cargo test -p lisa-plugin recovery_ack
1 passed; 0 failed
```

The new test materializes a matching generation-1 `pane-10.ack`, deletes it
before scanning, asserts the legacy reservation/thread/transport facts would
claim a false owner, and confirms current explicit state stays deadline-bound
and unowned.

It then evaluates the original deadline, observes generation-2 recovery,
launches one fresh Codex command for the same ticket, proves repeated transition
polls do not relaunch, withholds recovery acknowledgment, and reaches retained,
alerted `RecoveryFailed` at the second deadline.

### Package and workspace

Passed:

```text
cargo test -p lisa-plugin
265 passed; 0 failed

cargo test --workspace
lisa-cli unit tests: 270 passed
atomic provider integration: 1 passed
lisa-core unit tests: 150 passed
lisa-plugin unit tests: 265 passed
doc tests: 0 failed
total executed tests: 686 passed; 0 failed
```

### Quality and target checks

Passed after formatting:

```text
cargo fmt --all -- --check
cargo clippy -p lisa-plugin --all-targets -- -D warnings
cargo check -p lisa-plugin --target wasm32-wasip1
git diff --check -- crates/lisa-plugin/src/lib.rs
```

The ticket-owned source diff contains the new test plus one behavior-neutral
rustfmt normalization in the dependency's deadline expression.

## Commit log

The installed `lisa` binary did not recognize `commit-ticket`. As planned, the
repository CLI fallback succeeded:

```text
cargo run -p lisa-cli -- commit-ticket \
  --ticket-id T-033-03-01 \
  --message "test: reproduce dropped Codex handoff acknowledgment" \
  --include crates/lisa-plugin/src/lib.rs
```

Commit:

```text
d48f3f51a3bf975bd7b2c5076033a0ac69696c13
test: reproduce dropped Codex handoff acknowledgment
```

The commit contains exactly `crates/lisa-plugin/src/lib.rs`. After commit:

- the ticket-owned source path is clean;
- the ordinary index is empty;
- the ticket and work artifacts remain untracked for Lisa's completion
  transaction;
- unrelated modified and untracked worktree content remains untouched;
- ticket phase/status were not edited by this agent.
