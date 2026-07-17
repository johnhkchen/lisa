# Progress — T-048-02-02 ask authoring and auto-recheck

## Current state

Implementation started after completing Research, Design, Structure, and Plan.

## Completed

- Read repository and RDSPI workflow guidance.
- Mapped the structured disposition, parking projection, safe native check
  runner, CLI command surface, scheduler timer, host command boundary, and
  Unpark provenance reconciliation.
- Recorded the selected design and exact file structure.
- Defined three isolated source commit units.
- Expanded the rendered Review workflow with the complete structured block
  schema, honest owner rule, externally observable check rule, one-sentence ask
  rule, and exact Pages release counter-example.
- Extended template tests to pin every required instruction and retained raw /
  checked-in workflow byte equality.
- Verified all 31 filtered template tests and formatting.
- Committed unit 1 through Lisa's isolated transaction:
  `5f26e9c89eb8c0e257c2647c1e8f67077a920e10`.
- Added hidden `recheck-world` automation that independently filters for
  World-owned remedies with checks and reuses the native read-only runner.
- Added black-box fixtures for passing, failing, operator-owned, mutating, and
  timed-out automatic checks. All 12 parked UX fixtures and 5 low-level check
  tests pass; strict `lisa-cli` all-target Clippy passes.
- Committed unit 2 through Lisa's isolated transaction:
  `e7d21f819f5e5994c2954c69426141cd43be9bf7`.
- Added one plugin in-flight guard, exact native argv construction, startup
  invocation, five-second poll invocation, attributed result handling, DAG
  rebuild, Unpark reconciliation, and ordinary reseating.
- Added five native scheduler fixtures covering eligibility, boundaries,
  startup/cadence, overlap suppression, successful Unpark/reseat, idempotency,
  and no-churn failure.
- Verified all 408 plugin tests and workspace checking.
- Committed unit 3 through Lisa's isolated transaction:
  `5527142d3e9d55013a6541638f00f7e69d896bcc`.
- Final `cargo fmt --all -- --check` passed.
- Final `cargo test --workspace --no-fail-fast` passed across every unit,
  integration, and doc-test target.
- Final `git diff --check` passed.
- Audited all three commit path sets; each contains only its declared exact
  ticket-owned files.
- Confirmed the ordinary index has no staged entries and all ticket-owned
  source files are clean.

## In progress

- Review artifacts.

## Remaining

- Complete Review artifacts.

## Deviations

- Strict `cargo clippy -p lisa-plugin --all-targets -- -D warnings` reaches the
  pre-existing committed `emit_review_block_transition` helper from
  T-048-01-02 and reports `clippy::too_many_arguments`. This ticket did not
  change that API. `cargo check --workspace` and the complete plugin suite pass.
