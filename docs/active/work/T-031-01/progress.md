# Progress: T-031-01 isolated commit transaction

## Status

Implementation started from the approved Plan after completing Research, Design,
and Structure in this continuous RDSPI pass.

## Completed

- [x] Read `CLAUDE.md`, `AGENTS.md`, the RDSPI workflow, story S-031, and the
  ticket at its actual suffixed filename.
- [x] Mapped current artifact-driven completion, host command permissions,
  native CLI boundary, and existing diagnostic-only lock path.
- [x] Documented Git alternate-index/ref/index-reconciliation constraints.
- [x] Chose a native provider-neutral CLI transaction with explicit pathspecs.
- [x] Defined file/module structure and ordered implementation/test plan.

## Completed implementation

- [x] Added the native transaction module and `fs2` locking dependency.
- [x] Implemented request/path validation and actionable Git process errors.
- [x] Implemented full-section `.lisa-commit.lock` acquisition and release.
- [x] Implemented unique alternate-index creation and explicit cleanup.
- [x] Implemented ordinary staged path/entry snapshot and exact verification.
- [x] Implemented tree creation, commit creation, guarded ref update, and targeted
  ordinary-index reconciliation.
- [x] Added process-level isolation and failure regressions.
- [x] Wired the provider-neutral `commit-ticket` CLI command.
- [x] Verified the CLI help/argument contract.
- [x] Ran focused and workspace verification.

## In progress

- [ ] Inspect final diff and create an exact-path incremental commit.

## Remaining

- [ ] Inspect final diff and write `review.md`.

## Deviations

- The prompt named `docs/active/tickets/T-031-01.md`; the repository stores the
  ticket as `T-031-01-isolated-commit-transaction.md`. Work uses the ticket ID's
  standard artifact directory and does not rename or edit the ticket.
- No implementation deviations yet.

## Verification completed

- `cargo test -p lisa-cli commit_transaction`: 7 passed.
- `cargo clippy -p lisa-cli --bin lisa -- -D warnings`: passed.
- `cargo test --workspace`: passed (262 CLI, 145 core, 234 plugin tests).
- `just check`: passed, including `wasm32-wasip1` plugin check and all workspace
  tests.
- `cargo run -q -p lisa-cli -- commit-ticket --help`: passed and displayed all
  required arguments.
- `cargo clippy -p lisa-cli --all-targets -- -D warnings`: blocked by an existing
  `needless_borrows_for_generic_args` warning in `init.rs:2032`, outside this
  ticket's changes. Production-target Clippy is clean.

## Test correction

The first focused run passed six tests and failed only because the fixture's
success-asserting Git helper was used for an intentionally absent
`HEAD:unrelated.txt`. The assertion was changed to inspect the raw exit status;
the transaction state itself was correct. All subsequent focused and workspace
runs pass.

## Commit note

The repository contains unrelated modified and untracked user files. Any
incremental commit must be exact-path scoped and must not consume or rewrite
those changes. Source implementation will be verified before deciding whether a
safe incremental commit is appropriate.
