# Progress: T-031-02 gate done on commit

## Status

Research, Design, Structure, and Plan are complete. Implementation is beginning
in the same continuous RDSPI pass.

## Completed

- [x] Read project instructions, RDSPI workflow, ticket, story, and T-031-01
  implementation/review artifacts.
- [x] Located the ticket at its actual suffixed filename.
- [x] Mapped all automatic, idle, stopped, manual, generic sweep, provenance,
  dependent scheduling, and termination paths around Done.
- [x] Mapped the native isolated transaction and Zellij host-command boundary.
- [x] Chose a native preparation wrapper plus plugin pending-completion state.
- [x] Defined file-level structure and ordered implementation/test plan.

## In progress

- [ ] Run workspace/WASM release verification and write Review handoff.

## Remaining

- [x] Native `complete-ticket` wrapper and process regressions.
- [x] Plugin pending state, request/result handling, and DAG mask.
- [x] Route every Done trigger through the state machine.
- [x] Add automatic, timeout/finish-up, manual, reused Codex, dependent, and
  provenance regressions.
- [ ] Run focused/workspace/WASM/Clippy verification.
- [ ] Commit meaningful units with exact path scope.
- [ ] Write Review handoff.

## Implementation completed

- [x] Added `ticket::update_ticket_done`, which transforms phase/status in
  memory and performs one write.
- [x] Added `CompleteTicketRequest` and `complete_ticket`, preserving exact
  original ticket bytes on transaction failure.
- [x] Added `lisa complete-ticket` with explicit real ticket/work paths.
- [x] Added `pending_completions` with source, prior phase, and prior status.
- [x] Added argv-based host command construction and attributed result handling.
- [x] Masked pending Done frontmatter during DAG rebuild.
- [x] Preserved pending threads/slots in stale-slot and orphan audits.
- [x] Removed direct plugin Done writes from artifact, idle, stopped, and manual
  completion paths.
- [x] Replaced generic observed-Done teardown with the shared transaction.
- [x] Centralized success publication and failure recovery.
- [x] Added exact-once provenance and dependent-boundary coverage.

## Verification completed so far

- Baseline `cargo test -p lisa-core ticket::tests`: 30 passed.
- Baseline `cargo test -p lisa-cli commit_transaction`: 8 passed.
- Baseline `cargo test -p lisa-plugin auto_complete`: 7 passed.
- Final focused core ticket tests: 32 passed.
- Final CLI transaction tests: 10 passed.
- Final plugin suite: 236 passed.
- `cargo clippy -p lisa-plugin --all-targets -- -D warnings`: passed.
- `cargo check -p lisa-plugin --target wasm32-wasip1`: passed during compile
  iteration; the required release build remains in final verification.
- `cargo run -q -p lisa-cli -- complete-ticket --help`: passed.

## Commits

- `2cd5089fe31e2956cdaa719b9f9252ae05911abb` — combined completion
  frontmatter helper and tests.
- `52da2643d4cd9c6d73ada3074fa84b55efae91ee` — native atomic completion
  command and process regressions.

## Deviations

- The prompt names `docs/active/tickets/T-031-02.md`; the repository stores
  `docs/active/tickets/T-031-02-gate-done-on-commit.md`. The real discovered path
  is used and the ticket is not renamed.

## Worktree caution

The repository contains unrelated modified and untracked user files. All edits,
verification, and commits for this ticket will be path-scoped. No broad add,
commit, reset, checkout, or cleanup operation is permitted.
