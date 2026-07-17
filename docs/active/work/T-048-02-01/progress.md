# Progress — T-048-02-01 status-and-unblock-ux

## Current state

Implementation is in progress.

Research, Design, Structure, and Plan artifacts are complete in the current
attempt-private work directory.

No source file had been modified when the focused baseline suite ran.

## Baseline results

- core disposition tests: 14 passed;
- CLI status/preownership-status filtered tests: 11 passed;
- CLI help-surface integration tests: 6 passed;
- plugin UI tests: 47 passed.

## Planned source units

### Unit 1

Shared parked-remedy discovery and matching status/dashboard lines.

Status: committed.

Changes:

- added `lisa_core::parking::ParkedRemedy` and deterministic discovery;
- added core coverage for operator/world structure, legacy fallback, filtering,
  and sorting;
- made `lisa status` print operator/world asks before DAG mechanics;
- added dashboard `WaitingItem` projection from durable DAG/work state;
- added the Waiting on you section as first Operations content;
- added UI coverage for exact asks, world self-check copy, omission of owner/schema
  vocabulary, empty state, and ordering.

Focused results:

- core parking tests: 3 passed;
- core check: passed;
- CLI status/preownership-status filtered tests: 12 passed;
- plugin UI tests: 50 passed;
- plugin check: passed;
- exact-path diff check: passed.

Commit:

- `26ef88ae0b7a7bb4172b09eec7b68fd119bb1b2e`;
- subject `T-048-02-01: show parked asks in status and dashboard`;
- committed through `lisa commit-ticket` with five exact include paths.

Deviation:

The first plugin compile found one complete `PluginState` test literal that did
not use `..Default`. It was updated with an empty waiting vector and the rerun
passed. The initial renderer colored the ticket ID, which placed an ANSI reset
between ID and ask and made the semantic line harder to assert exactly. ID
coloring was removed; the section heading retains dashboard styling and the
ask line is now plain text.

### Unit 2

Disposable read-only check runner, unblock command, help surface, and black-box
operator fixtures.

Status: committed.

Changes:

- added `lisa unblock <id> --path <project>` as a visible operator command;
- added explicit Reopened/Declined outcomes so expected declines have no generic
  error prefix;
- added configured ticket/work discovery and canonical block validation;
- added optional check execution with a five-second deadline;
- added disposable Git-visible/non-Git project snapshots;
- removed write bits and fingerprinted snapshot paths, modes, and contents;
- redirected output to anonymous temporary files so it cannot block polling;
- started the shell in a Unix process group and kills the group on timeout;
- reduced failure output to one sanitized, capped observation line;
- detects writes even when a check changes permissions first;
- updates only the real ticket status and only after a clean pass/no-check;
- added exact help snapshots and command inventory/order coverage;
- added seven real-binary parked UX fixtures.

Focused results:

- unblock safety unit tests: 5 passed;
- parked UX binary fixtures: 7 passed;
- help-surface integration tests: 6 passed;
- status/preownership-status filtered tests: 12 passed;
- core parked-remedy tests: 3 passed;
- plugin UI tests: 50 passed;
- dashboard canonical-projection test: 1 passed;
- CLI check: passed.

Commit:

- `16c9a2da083cec7226c2f1d620d85adb6b5df0d9`;
- subject `T-048-02-01: verify and reopen parked tickets`;
- committed through `lisa commit-ticket` with six exact include paths.

The first check-runner invocation was made before `mod unblock` was wired and
therefore matched zero tests. After main wiring, all five runner tests executed
and passed. One observation test initially used escaped text rather than an
actual ANSI control byte; the fixture was corrected. The sanitizer was then
strengthened to remove full ANSI CSI sequences instead of leaving `[31m` text.

### Projection boundary follow-up

Status: committed.

After Unit 1 committed, an additional production-boundary fixture was added to
prove `State::to_ui_state` reads a canonical disposition for a durable blocked
ticket rather than relying only on manually constructed UI state.

Commit:

- `6498006dcc0707912b97078e26898b3b629a7bbe`;
- subject `T-048-02-01: cover durable ask dashboard projection`;
- exact include path `crates/lisa-plugin/src/lib.rs`.

This is a documented sequencing deviation only; it adds test coverage and does
not change the chosen architecture or runtime behavior.

### Lint cleanup

Status: committed.

The strict CLI all-targets Clippy pass found three mechanical findings after
the full workspace suite:

- two single-byte array literals in the fingerprint hash;
- one needless generic-argument borrow in the binary fixture.

Both affected files were corrected, formatted, retested, and committed with
exact includes.

Commit:

- `b5618a12aa0567bb33f6f4950276d1a541aaaaac`;
- subject `T-048-02-01: keep unblock checks lint-clean`;
- exact includes `crates/lisa-cli/src/unblock.rs` and
  `crates/lisa-cli/tests/parked_ux.rs`.

Final strict result:

```text
cargo clippy -p lisa-cli --all-targets -- -D warnings
passed
```

## Complete verification

### Workspace compile

```text
cargo check --workspace
passed
```

### Complete workspace tests

```text
cargo test --workspace --no-fail-fast
passed
```

Notable totals:

- CLI library: 19 passed;
- CLI binary: 328 passed;
- parked UX integration: 7 passed;
- help surface integration: 6 passed;
- core unit suite: 219 passed;
- completion state machine: 1 passed;
- recorded livelock regression: 1 passed;
- plugin unit suite: 403 passed;
- all other executed CLI integration suites passed;
- real-Zellij delivery boundary remained ignored under its documented external
  tooling/target gate.

### Formatting

```text
cargo fmt --all -- --check
passed
```

### Project quick check

```text
just check
passed
```

This reran:

- `cargo check -p lisa-plugin --target wasm32-wasip1`;
- `cargo test --workspace`.

The final quick check ran after the lint cleanup commit.

### Commit hygiene

Every ticket commit passes `git show --check`:

- `26ef88a` waiting surfaces;
- `6498006` dashboard canonical projection fixture;
- `16c9a2d` unblock command and safety boundary;
- `b5618a1` strict-lint cleanup.

All 11 ticket-owned source paths are clean.

`git diff --cached --name-only` is empty.

The remaining worktree changes are Lisa-managed ledgers/ticket phase updates and
unrelated concurrent work artifacts that existed outside this ticket's source
include sets.

No ordinary staging or commit command was used.

## Implementation outcome

All acceptance behaviors are implemented and verified.

No source work remains.

Review artifacts are next.

## Ownership constraints

The worktree contained unrelated modified Lisa ledgers/tickets and untracked
work directories before implementation.

They remain outside this ticket's include set.

No ordinary Git staging or commit command will be used.
