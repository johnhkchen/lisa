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

- [x] Review handoff written.

## Remaining

- [x] Native `complete-ticket` wrapper and process regressions.
- [x] Plugin pending state, request/result handling, and DAG mask.
- [x] Route every Done trigger through the state machine.
- [x] Add automatic, timeout/finish-up, manual, reused Codex, dependent, and
  provenance regressions.
- [x] Run focused/workspace/WASM/Clippy verification.
- [x] Commit meaningful units with exact path scope.
- [x] Write Review handoff.

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
- `e85b31348082cfc55b0f61f79c0bda2f5acdc332` — plugin completion state
  machine, routed triggers, regressions, and pre-review RDSPI artifacts.
- `b8903cd64b4d88bd88a27240809d3db313088550` — compensating rollback for
  failures after the completion commit advances `HEAD`.
- `ef5aa39eb93ef05f00583e0b4d8777bdf222561c` — idempotent verification for
  externally committed Done tickets.

## Final verification

- `cargo test -p lisa-core ticket::tests`: 32 passed at focused checkpoint.
- `cargo test -p lisa-cli commit_transaction`: 12 passed after final edge-case
  coverage.
- `cargo test -p lisa-plugin`: 236 passed.
- `cargo clippy -p lisa-plugin --all-targets -- -D warnings`: passed.
- `cargo clippy -p lisa-cli --bin lisa -- -D warnings`: passed.
- `cargo build -p lisa-plugin --target wasm32-wasip1 --release`: passed.
- `cargo test --workspace`: passed.
- Final `just check`: passed:
  - WASM development check passed;
  - 267 CLI tests passed;
  - 147 core tests passed;
  - 236 plugin tests passed;
  - doc tests passed.
- `git diff --check` on ticket-owned work artifacts: passed.

## Final deviations and corrections

- The design initially treated T-031-01 post-ref cleanup errors as an inherited
  ambiguity. Self-review identified that restoring non-Done working bytes over
  an advanced Done commit would make retries unsafe. The transaction now
  compensates by rolling back `HEAD` and reconciling exact ticket paths before
  reporting failure; a process test covers the rollback boundary.
- Externally committed Done tickets were initially routed to `complete-ticket`
  but could return "no changes" forever. The command now verifies clean explicit
  ticket/work paths and returns the current commit ID as an idempotent success.
- Lisa automatically advanced the ticket phase while artifacts appeared. This
  session did not edit the ticket's phase or status fields.

## Deviations

- The prompt names `docs/active/tickets/T-031-02.md`; the repository stores
  `docs/active/tickets/T-031-02-gate-done-on-commit.md`. The real discovered path
  is used and the ticket is not renamed.

## Worktree caution

The repository contains unrelated modified and untracked user files. All edits,
verification, and commits for this ticket will be path-scoped. No broad add,
commit, reset, checkout, or cleanup operation is permitted.
