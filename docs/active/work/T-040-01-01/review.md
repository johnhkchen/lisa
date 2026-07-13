# Review: Review disposition emission contract

## Disposition

Pass. T-040-01-01 meets its acceptance criterion and leaves the filename and
shape explicit for T-040-01-02.

## Summary

The Review phase now requires two companion artifacts: the human-readable
`review.md` and the machine-readable `review-disposition.json`. The contract is
identical in the repository workflow documentation and the CLI's embedded
outgoing workflow.

The fixed path is:

```text
docs/active/work/{ticket-id}/review-disposition.json
```

The canonical passing document is:

```json
{"disposition":"pass","reason":null}
```

The canonical blocking document is:

```json
{"disposition":"block","reason":"<non-empty actionable reason>"}
```

The instructions explicitly reject a pass carrying a reason and a block without
a non-empty reason. This resolves the nullability and contradiction behavior the
successor parser ticket otherwise would have had to infer.

## Files modified

### `docs/knowledge/rdspi-workflow.md`

The Review section now:

- directs agents to write `review-disposition.json` beside `review.md`;
- shows exact pass and block JSON shapes;
- defines the validity relationship between `disposition` and `reason`;
- makes waiting conditional on writing both Review artifacts;
- lists both active-work artifact paths.

No other workflow phase or rule changed.

### `crates/lisa-cli/data/rdspi-workflow.md`

The embedded outgoing workflow received the identical Review text. A byte-level
comparison confirms this file and the project documentation are equal. Fresh or
safely upgraded Lisa projects therefore receive the same contract used in this
repository.

Historical workflow templates under `data/legacy` were intentionally preserved;
they remain exact recognition inputs for safe upgrades.

### `crates/lisa-cli/src/templates.rs`

The broad embedding test now asserts Review is present. A new focused test,
`test_review_disposition_contract_is_injected`, verifies both sides of the
agent-contract chain:

- generated `CLAUDE.md` points to `docs/knowledge/rdspi-workflow.md` and states
  that Lisa injects it;
- embedded `RDSPI_WORKFLOW` contains the fixed disposition filename;
- embedded workflow text contains the exact pass payload;
- embedded workflow text contains the exact block payload example;
- embedded workflow text contains the contradiction rule.

No production Rust API or code path changed in `templates.rs`; the existing
`include_str!` boundary remains intact.

## Acceptance criterion assessment

The documented Review section specifies a fixed filename: met.

The embedded agent contract specifies the same fixed filename: met through the
updated `crates/lisa-cli/data/rdspi-workflow.md` consumed by
`templates::RDSPI_WORKFLOW`.

Both specify the JSON object keys and pass/block variants: met.

Both define reason semantics rather than leaving contradictory input ambiguous:
met. Pass requires JSON null; block requires a non-empty actionable string.

A templates test asserts generated/injected contract reachability and content:
met.

The shape is settled for the parser ticket: met. T-040-01-02 can deserialize the
two fields and validate their relationship without choosing a filename or
reason representation.

## Verification evidence

### Focused contract test

```text
cargo test -p lisa-cli templates::tests::test_review_disposition_contract_is_injected
```

Passed: 1 passed, 0 failed.

### Complete CLI suite

```text
cargo test -p lisa-cli
```

Passed:

- 276 unit tests;
- 1 atomic-provider integration test;
- 3 help-surface integration tests;
- 0 failures.

The real-Zellij integration test remained ignored by its own environment guard.
This ticket does not touch Zellij behavior.

### Workspace suite

```text
cargo test --workspace
```

Passed all runnable tests across `lisa-cli`, `lisa-core`, and `lisa-plugin`, with
zero failures. The same explicitly environment-gated real-Zellij test was
ignored.

### Formatting and static checks

- `cargo fmt --all -- --check`: passed after applying rustfmt's assertion
  wrapping.
- `git diff --check`: passed before the source transaction.
- `cmp` between current workflow copies: passed.
- Exact diff review: 37 insertions and 4 deletions across only the three owned
  files.

## Commit evidence

The meaningful source unit was committed through Lisa's isolated transaction:

```text
ce59acb5ce0464ec087091cbc13cb97efb18ab71
```

Commit subject: `Document Review disposition emission contract`.

Exact included paths:

- `docs/knowledge/rdspi-workflow.md`;
- `crates/lisa-cli/data/rdspi-workflow.md`;
- `crates/lisa-cli/src/templates.rs`.

The installed `/opt/homebrew/bin/lisa` predates `commit-ticket` and rejected the
subcommand before mutation. The repository-built, fully tested
`target/debug/lisa` executed the required isolated transaction with the same
exact includes. No ordinary `git add`, ordinary `git commit`, or broad staging
operation was used.

All three ticket-owned paths were clean immediately after the transaction.
Unrelated scheduler-owned ticket/work paths and concurrent T-040-02 source work
remain outside this ticket and were not included.

## Coverage assessment

Coverage is proportional to the ticket's contract-only scope. Exact-string tests
are appropriate here because the filename and JSON spelling are the behavior
being pinned. Full CLI tests exercise template initialization and safe-upgrade
paths around the changed embedded body. Workspace tests establish that the text
change does not disturb core or plugin builds/tests.

No parser test is included because no parser exists in this ticket. The next
ticket is responsible for pass, block, missing, malformed, and contradictory
input cases. No scheduler gate test is included because T-040-01-03 owns that
runtime seam.

## Open concerns and limitations

The scheduler does not yet enforce this disposition. Until T-040-01-02 and
T-040-01-03 land, current completion behavior can still treat `review.md`
existence as sufficient. This is expected dependency-chain state, not an
unimplemented part of T-040-01-01.

The contract does not state whether unknown additional JSON keys are rejected.
Canonical agent output contains only `disposition` and `reason`; strictness for
extra keys remains a parser implementation choice. This does not affect the
named acceptance cases.

The Review prose says “exactly one of these JSON shapes,” which makes canonical
emission clear. The block example uses a placeholder string and separately
requires a non-empty actionable reason, avoiding any risk that the placeholder
itself is mistaken for an accepted literal.

No critical issue requires human attention. The ticket is ready for Lisa's
completion publication and for T-040-01-02 to consume the settled contract.
