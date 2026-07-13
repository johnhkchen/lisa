# Progress: Review disposition emission contract

## Status

Implementation complete, verified, and committed through Lisa's isolated
transaction.

## Completed work

- Updated `docs/knowledge/rdspi-workflow.md` Review instructions.
- Updated the embedded outgoing copy at
  `crates/lisa-cli/data/rdspi-workflow.md` with identical text.
- Fixed the companion filename as `review-disposition.json`.
- Defined the canonical pass payload as
  `{"disposition":"pass","reason":null}`.
- Defined the canonical block payload as
  `{"disposition":"block","reason":"<non-empty actionable reason>"}`.
- Documented that pass with a reason and block without a non-empty reason are
  invalid.
- Changed the Review wait boundary to require both artifacts.
- Listed both active-work artifact paths.
- Extended the embedded phase test to include Review.
- Added `test_review_disposition_contract_is_injected` in `templates.rs`.
- Verified generated `CLAUDE.md` retains the documented injection pointer.
- Pinned the filename, both JSON shapes, and contradiction rule in exact string
  assertions against `RDSPI_WORKFLOW`.

## Verification results

### Workflow parity

Command:

```text
cmp docs/knowledge/rdspi-workflow.md crates/lisa-cli/data/rdspi-workflow.md
```

Result: pass; files are byte-identical.

### Formatting

The initial `cargo fmt --all -- --check` reported only assertion wrapping in the
new Rust test. `cargo fmt --all` applied the canonical layout. The final
`cargo fmt --all -- --check` passed.

This formatting pass does not change behavior or scope.

### Focused test

Planned command:

```text
cargo test -p lisa-cli templates::tests::test_review_disposition_contract_is_injected
```

Result: pass; 1 passed, 0 failed.

### CLI suite

Command:

```text
cargo test -p lisa-cli
```

Result: pass.

- 276 unit tests passed;
- 1 atomic provider contract integration test passed;
- 3 help-surface integration tests passed;
- 1 real-Zellij test remained ignored because it requires external runtime
  dependencies, as declared by the test;
- 0 failures.

### Workspace suite

Command:

```text
cargo test --workspace
```

Result: pass; all runnable CLI, core, and plugin tests passed, with no failures.
The same explicitly environment-gated real-Zellij test remained ignored.

### Static checks

- `git diff --check`: pass.
- Exact diff inspection: only the three intended files contain ticket-owned
  edits.
- Current and embedded workflow files remain identical.
- Legacy workflow files have no ticket diff.

## Scope confirmation

No parser was added to `lisa-core`; T-040-01-02 owns that work. No scheduler or
completion behavior was changed; T-040-01-03 owns gating. No ticket phase/status
field was edited by this implementation.

The worktree contains scheduler-owned phase changes and published work artifacts
for T-040-01-01 and T-040-02-01. Those paths were observed but not edited or
included by this ticket.

## Source transaction

Planned exact include paths:

- `docs/knowledge/rdspi-workflow.md`;
- `crates/lisa-cli/data/rdspi-workflow.md`;
- `crates/lisa-cli/src/templates.rs`.

Command:

```text
lisa commit-ticket --ticket-id T-040-01-01 \
  --message "Document Review disposition emission contract" \
  --include docs/knowledge/rdspi-workflow.md \
  --include crates/lisa-cli/data/rdspi-workflow.md \
  --include crates/lisa-cli/src/templates.rs
```

No ordinary Git index operation has been used. Attempt-private artifacts are not
part of the source transaction.

The installed `/opt/homebrew/bin/lisa` rejected `commit-ticket` as an unknown
subcommand. This happened before any transaction or index mutation. Per the
planned fallback, the repository-built `target/debug/lisa` from the verified
workspace was invoked with the same exact arguments.

Result: commit `ce59acb5ce0464ec087091cbc13cb97efb18ab71`.

## Remaining

1. Confirm the three owned paths are clean.
2. Write `review.md`.
3. Emit this ticket's own `review-disposition.json`.
4. Stop on the current ticket for Lisa completion processing.
