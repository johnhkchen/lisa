# Progress — T-046-01-01 version parse and supported range

## Status

Implementation, verification, and exact-path source commits are complete.
Review artifacts remain.

## Completed

- Read `AGENTS.md`, `CLAUDE.md`, the ticket, story, dependent ticket, epic, and
  RDSPI workflow.
- Mapped the existing `lisa-core` module and dependency conventions.
- Confirmed `zellij-tile = "0.43"` is declared in
  `crates/lisa-plugin/Cargo.toml`.
- Confirmed doctor currently retains `zellij --version` output as opaque text.
- Confirmed semver 1.0.27 is already present transitively in `Cargo.lock` and
  offers const release construction.
- Wrote `research.md`.
- Wrote `design.md`.
- Wrote `structure.md`.
- Wrote `plan.md`.
- Added `semver = "1.0"` as a direct `lisa-core` dependency.
- Added the public `lisa_core::version` module.
- Added the semver-backed comparable `ZellijVersion` newtype.
- Added strict `zellij <semver>` command-output parsing.
- Added `ZellijVersionRange` and the open-ended
  `SUPPORTED_ZELLIJ_RANGE` with floor 0.43.0.
- Documented the `zellij-tile = "0.43"` pin rationale and distinct 0.41.0
  theoretical protocol floor beside the range constant.
- Added the `InRange`, `BelowFloor`, and `Unparseable` verdict API.
- Added seven unit tests covering stable releases, below-floor releases,
  prereleases, malformed output, semantic ordering, and display.
- Ran `cargo fmt --all -- --check` successfully after applying rustfmt's line
  wrapping.
- Ran `cargo test -p lisa-core version`: 7 passed, 0 failed.
- Ran `cargo test -p lisa-core`: 207 unit tests and 2 integration tests passed,
  0 failed.
- Ran `cargo test --workspace`: all workspace tests passed; the existing real
  Zellij delivery boundary remained ignored by its declared environment gate.
- Ran `just check`: WASM target check and the repeated workspace suite passed.
- Ran `git diff --check` on the four source paths successfully.
- Committed the implementation through `lisa commit-ticket` as
  `5479aa75dda4533a836df73b3d57152242faf218`.
- Audited that commit and found a concurrent `directories` dependency edge had
  entered `Cargo.lock` while the isolated transaction took its snapshot.
- Removed only that foreign lockfile edge and committed the correction through
  `lisa commit-ticket` as `2edf4e367460fde25429b04ad807f15cf264f8a0`.
- Verified the net diff from the pre-ticket parent through the correction has
  exactly the four planned paths and only the intended `semver` lockfile edge.
- Verified the ordinary Git index has no staged paths.

## Remaining

- Write Review artifacts and disposition.

## Planned source ownership

- `crates/lisa-core/src/version.rs`
- `crates/lisa-core/src/lib.rs`
- `crates/lisa-core/Cargo.toml`
- `Cargo.lock`

## Deviations

- The first `cargo fmt --all -- --check` reported only line-wrapping changes in
  the new tests. `cargo fmt --all` applied those mechanical changes, and the
  formatting check then passed.
- A concurrent T-046-02-03 implementation modified `crates/lisa-cli` and added
  `directories` to the shared `Cargo.lock` while this ticket's first isolated
  commit was taking its snapshot. The foreign lockfile line was removed in a
  second exact-path Lisa commit. The other ticket has since recreated its own
  uncommitted lockfile hunk from its still-modified CLI manifest; that hunk is
  not part of this ticket's net commits and was intentionally left untouched.
- No design, structure, API, or test-strategy deviation was required.
