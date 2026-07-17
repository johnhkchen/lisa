# Progress — T-050-03-01 client-autodetect

## Current state

Implementation and aggregate verification are complete.
Research, Design, Structure, and Plan artifacts are complete.
Filesystem-only agent availability detection is implemented in `detect.rs`.
Configuration resolution is implemented on top of the config catalog committed by `T-050-02-01`.
Doctor and real loop startup announce the selected client and reason.
The controlled-PATH integration fixture covers the full detection matrix and override precedence.

## Completed work

- Added four explicit PATH availability states: neither, Claude only, Codex only, and both.
- Added direct PATH-directory inspection without `which` or provider execution.
- Added Unix executable-bit validation.
- Added Windows PATHEXT candidate handling.
- Added pure classification coverage for all four states.
- Added Unix coverage distinguishing regular files from executable files.
- Prototyped typed client-resolution provenance and exact announcements.
- Verified the prototype across all config availability and precedence cases before removing it from the colliding file.
- Reapplied typed client-resolution provenance after the concurrent catalog commit.
- Added exact brand-voiced announcements for detected, configured, and CLI-selected clients.
- Added the announcement to doctor output and real loop startup only.
- Added seven real-binary controlled-PATH fixture tests.
- Preserved the existing neither-installed Claude remedy as an exact asserted substring.
- Committed all ticket-owned source through Lisa's isolated transaction.

## Verification completed

- `cargo test -p lisa-cli --bin lisa config::tests`: 65 passed.
- `cargo test -p lisa-cli --bin lisa detect::tests`: 9 passed.
- `cargo fmt --all`: completed.
- `git diff --check -- crates/lisa-cli/src/detect.rs crates/lisa-cli/src/config.rs`: passed before the ownership collision was identified.
- `cargo test -p lisa-cli --bin lisa config::tests`: 65 passed after the final rebase.
- `cargo test -p lisa-cli --bin lisa doctor::tests`: 50 passed.
- `cargo test -p lisa-cli --bin lisa loop_cmd::tests`: 24 passed.
- `cargo test -p lisa-cli --test client_autodetect`: 7 passed.
- `cargo fmt --all`: passed on the completed source.
- `cargo test --workspace`: passed across CLI, core, plugin, and integration suites.
- `just check`: passed the `wasm32-wasip1` plugin check and repeated workspace tests.
- `git diff --cached --name-only`: empty; the ordinary index contains no ticket work.
- `git status --short`: all five ticket-owned source/test paths are clean.
- `git diff --check HEAD`: passed for remaining scheduler/concurrent-ticket worktree changes.

## Plan deviation

The planned first commit included `detect.rs` and `config.rs` together.
During the diff check, `config.rs` contained a large uncommitted config-catalog change owned by another active ticket.
An exact-path ticket commit would have consumed both tickets' work.
To preserve isolated ownership, this attempt removed only its config edits and split PATH detection into its own durable commit.
After the catalog ticket commits, client resolution will be reapplied as a separate meaningful unit.
No ticket requirement or architectural decision changed because of this sequencing deviation.
`T-050-02-01` committed its catalog/upsert unit as `363e82d`; this ticket then reapplied cleanly.

## Ticket source commits

- `5e91e5c` — Add PATH-only agent availability detection (`detect.rs`).
- `47e7336` — Resolve unconfigured clients from PATH (`config.rs`).
- `d88cd13` — Announce detected clients in doctor and loop (`doctor.rs`, `loop_cmd.rs`, integration fixture).

## Remaining work

- Complete Review artifacts and validate disposition.
