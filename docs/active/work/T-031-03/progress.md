# Progress: T-031-03 provider contract and live regression

## Status

Research, Design, Structure, and Plan are complete. Implementation is in
progress in the same continuous RDSPI pass.

## Completed

- [x] Read `CLAUDE.md`, `AGENTS.md`, the RDSPI workflow, ticket, and S-031 story.
- [x] Read the T-031-01 and T-031-02 implementation/review boundaries.
- [x] Mapped isolated transaction, completion state machine, provider prompts,
  workflow ownership rules, user docs, and existing regression coverage.
- [x] Chose explicit isolated implementation commits plus the authoritative
  scheduler completion transaction.
- [x] Defined file structure and ordered implementation/test plan.

## In progress

- [x] Review handoff written.

## Remaining

- [x] Preserve and update the bundled workflow contract.
- [x] Register ownership-aware upgrade history and tests.
- [x] Align common initial/reuse/finish-up prompts and tests.
- [x] Resolve descriptive ticket filenames in provider prompts.
- [x] Add and run the six-ticket external-repository harness.
- [x] Add Cargo integration coverage for the harness.
- [x] Update README and setup/recovery documentation.
- [x] Run focused and required broad verification.
- [x] Record implementation commits and deviations.
- [x] Write Review handoff.

## Plan deviation: real ticket path in provider prompt

Implementation inspection found that `ticket_prompt` always formats
`<ticket-dir>/<ticket-id>.md`, while active tickets commonly use descriptive
filenames such as `T-031-03-provider-contract-live-regression.md`. The scheduler
already discovers and retains the real `Ticket.file_path` for atomic completion,
but initial and reuse prompts can name a nonexistent path. Because the ticket
requires the exact provider contract and the live fixture uses descriptive
filenames, implementation will resolve the real path from the ticket scan with
the short path retained only as a fallback for isolated prompt tests. A focused
regression will cover this behavior.

## Implementation completed

- [x] Replaced generic workflow Git instructions with exact-path
  `lisa commit-ticket` guidance and ordinary-index prohibitions.
- [x] Made Review explicitly wait for Lisa's completion confirmation.
- [x] Added the completion failure/seat/dependent recovery rule.
- [x] Preserved the exact outgoing six-phase workflow as a known legacy template.
- [x] Extended init ownership tests across every known workflow generation.
- [x] Kept unknown/customized installed workflows on the S-030 safety-skip path.
- [x] Added atomic contract language to common initial and reuse prompts.
- [x] Added the same terminal wait rule to common finish-up prompts.
- [x] Added real descriptive-ticket path discovery with short-name fallback.
- [x] Added phrase-level prompt and descriptive-path regressions.
- [x] Added an external Git harness with five Codex tickets, one Claude ticket,
  a reused logical seat, dependency edges, and a foreign staged file.
- [x] Added commit-tree, index, activity, provenance, status, init, and validate
  evidence capture outside the fixture repository.
- [x] Added a Cargo integration test that runs the harness with the built binary.
- [x] Corrected README and setup guide workflow, atomicity, and recovery text.

## Focused verification

- Workflow legacy fixture exactly matched the outgoing committed bundle.
- Bundled and repository-installed current workflow copies are byte-identical.
- Harness script passes `bash -n`.
- `test_plan_init_updates_every_known_rdspi_template`: 1 passed.
- `test_plan_init_preserves_unknown_rdspi`: 1 passed.
- `test_ticket_prompt*`: 3 passed.
- `test_finish_up_prompt_preserves_atomic_completion_contract`: 1 passed.
- `atomic_provider_contract` integration test: 1 passed.
- Retained direct harness run: PASS with six confirmed completion receipts.
- Retained run recorded five Codex starts and one Claude start on `seat-1`.
- Foreign stage tuple before/after was identical:
  `100644 e4199730783d26c93fe57610bd955b2bf3cf7248 0 foreign.txt`.
- Final fixture status contained only the expected staged `foreign.txt`.
- Evidence root for this implementation run:
  `/var/folders/kn/7f93dn8n1wb51m_jydvylncw0000gn/T/lisa-t03103.WQgnnS/evidence`.

## Implementation commits

- `2cb06893411e8248c51a4f548dba940f11db3ef7` — bundled/installed
  workflow, ownership history, common provider prompts, real ticket path, tests.
- `93671549e34641a8cdc615145ce5b5ba7261864a` — external-repository
  provider contract harness, evidence guide, Cargo integration test.
- `9edc92b66307c58b5601ac6a1fa93d7ce98ae401` — README and setup
  guide atomicity/recovery documentation.

All three commits were created with `lisa commit-ticket` and exact include paths.
The ordinary index remained empty and unrelated worktree changes were excluded.

## Required broad verification

- `cargo fmt --all -- --check`: passed.
- `cargo run -q -p lisa-cli -- validate`: passed; 16 tickets, 1 ready,
  valid DAG.
- `cargo clippy -p lisa-plugin --all-targets -- -D warnings`: passed.
- `cargo test --workspace`: passed.
  - Lisa CLI unit tests: 268 passed.
  - Atomic provider-contract integration: 1 passed.
  - Lisa core tests: 147 passed.
  - Lisa plugin tests: 238 passed.
  - Doc tests: passed.
- `cargo build -p lisa-plugin --target wasm32-wasip1 --release`: passed.
- `just check`: passed, including WASM development check and the full workspace
  suite with the external-repository harness.

## Final state

- All planned implementation and documentation changes are committed through
  exact-path isolated transactions.
- Only this ticket's RDSPI artifacts remain for Lisa's final completion commit.
- The session did not edit the ticket's phase or status frontmatter; Lisa
  advanced phase automatically as artifacts appeared.
- No ticket-caused test, lint, validation, or build failure remains.

## Worktree caution

The repository contains unrelated modified and untracked user files. Edits,
verification, and any commits for this ticket remain exact-path scoped. The
ordinary Git index must not be used for ticket staging.
