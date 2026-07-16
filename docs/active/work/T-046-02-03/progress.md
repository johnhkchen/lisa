# Progress: XDG-aware Zellij pre-grant and cache

## Status

- Phase: Implement.
- Implementation unit: Zellij-compatible cache resolution and routing regressions.
- Current state: implementation, verification, and isolated source commit complete.

## Completed

- Read the ticket, repository guidance, and RDSPI workflow.
- Mapped all production consumers of `zellij_cache_dir()`.
- Confirmed Zellij 0.43.1 uses `directories` 5.0.1 `ProjectDirs`.
- Confirmed the exact project tuple is `org`, `Zellij Contributors`, `Zellij`.
- Confirmed Linux absolute `XDG_CACHE_HOME` semantics in the resolved dependency source.
- Confirmed the existing macOS bundle-ID path in the resolved dependency source.
- Wrote Research, Design, Structure, and Plan artifacts.
- Ran focused cleanup baseline tests.
- Cleanup baseline result: 3 passed, 0 failed.
- Ran focused pre-grant baseline tests.
- Pre-grant baseline result: 7 passed, 0 failed.
- Added `directories = "5"` as a direct CLI runtime dependency.
- Updated the lockfile's `lisa-cli` dependency edge.
- Replaced manual `HOME` and platform branching with Zellij's exact `ProjectDirs` tuple.
- Preserved the private resolver's `Option<PathBuf>` interface.
- Preserved all three existing production consumers without call-site edits.
- Added scoped, unwind-safe restoration for `HOME` and `XDG_CACHE_HOME` in tests.
- Added a configured-environment wrapper regression.
- Added an unconfigured-environment wrapper regression.
- Both regressions assert the exact resolved cache path.
- Both regressions exercise recursive Lisa plugin-cache cleanup through the runtime wrapper.
- Both regressions exercise permission pre-grant through the runtime wrapper.
- The configured regression asserts the non-selected candidate receives no permission file.
- Linux-specific expectations cover `$XDG_CACHE_HOME/zellij` and `$HOME/.cache/zellij`.
- macOS-specific expectations cover the unchanged bundle-ID cache path.
- Ran `cargo fmt --all` successfully.
- Ran `cargo check -p lisa-cli` successfully.
- Ran `git diff --check` successfully.
- Ran the configured regression alone: 1 passed, 0 failed.
- Ran the unconfigured regression alone: 1 passed, 0 failed.
- Re-ran cleanup tests: 3 passed, 0 failed.
- Re-ran pre-grant tests: 7 passed, 0 failed.
- Ran `cargo test -p lisa-cli` successfully.
- CLI library unit tests: 14 passed, 0 failed.
- CLI binary unit tests: 272 passed, 0 failed.
- CLI integration tests: 13 passed, 0 failed, 1 environment-dependent test ignored.
- Ran `just check` successfully.
- WASM target check passed.
- Workspace native tests passed.
- Workspace doc tests passed.
- Committed the source unit through `lisa commit-ticket`.
- Commit: `485b7f78c3304537575571d76ba561bdf5390b1e`.
- Commit message: `fix(cli): match Zellij cache directory resolution`.
- Exact includes were `Cargo.lock`, `crates/lisa-cli/Cargo.toml`, and `crates/lisa-cli/src/doctor.rs`.
- Verified all three ticket-owned source paths are clean after the commit.
- Verified the ordinary Git index has no staged paths.

## In progress

- None.

## Remaining

- Complete Review artifacts.

## Deviations

- A concurrent ticket briefly committed the generated `lisa-cli -> directories` lockfile edge while editing the same shared lockfile.
- That ticket then removed the foreign edge in commit `2edf4e3`, returning ownership of the lockfile delta to this ticket.
- The final source unit therefore retains the planned three exact include paths.
- No implementation design or acceptance behavior changed.
