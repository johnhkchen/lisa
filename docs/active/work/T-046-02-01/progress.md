# Progress — T-046-02-01 runtime resolver and config

## Status

Implement phase started.

## Completed

- Read `CLAUDE.md`, the ticket, and the complete RDSPI workflow.
- Mapped `.lisa.toml` parsing, resolution, default generation, and init merge.
- Mapped doctor dependency/report behavior and loop launch behavior.
- Read the T-046-01-01 version contract and completion artifacts.
- Read adjacent T-046-01-02 Research and confirmed its active source overlap.
- Read T-046-02-02 and T-046-02-03 boundaries.
- Recorded Research, Design, Structure, and Plan artifacts privately.
- Chose a dedicated native runtime module and typed config request.

## Planned source paths

- `crates/lisa-cli/src/runtime.rs`;
- `crates/lisa-cli/src/config.rs`;
- `crates/lisa-cli/src/main.rs`;
- `crates/lisa-cli/src/loop_cmd.rs`;
- `crates/lisa-cli/src/doctor.rs`.

No manifest or lockfile change is currently expected.

## Concurrency note

T-046-01-02 is active in another Lisa seat.

It is expected to modify `doctor.rs` and `loop_cmd.rs`.

This ticket will implement the new runtime module and config wiring first.

Before touching the shared files, it will re-read HEAD and incorporate the
adjacent ticket's committed API.

No uncommitted foreign source path is currently present.

## Remaining

- Implement runtime request/result vocabulary and managed path derivation.
- Implement PATH lookup and version inspection.
- Extend config schema, resolution, validation, and template.
- Add focused tests.
- Reconcile the adjacent ticket's committed enforcement work.
- Wire loop to the resolved path.
- Wire doctor to runtime reporting.
- Run focused, package, and workspace verification.
- Commit ticket-owned source with exact Lisa transaction paths.
- Complete Review artifacts and disposition.

## Implementation update 1 — resolver and config

Created `crates/lisa-cli/src/runtime.rs`.

Implemented typed managed/system/pinned requests, stable mode labels, and a
resolved mode/version/absolute-path result.

Declared managed release 0.43.1 and implemented the exact XDG data path plus
HOME fallback.

Implemented ordered PATH lookup, absolute normalization, `--version`
inspection, core range classification, and fail-closed named errors.

Extended `.lisa.toml` with `[runtime].zellij`, managed absence default,
unknown-key warnings, absolute-pin validation, and documented template examples.

Declared the binary-local runtime module in `main.rs`.

Focused results:

- runtime tests: 9 passed, 0 failed;
- config tests: 57 passed, 0 failed.

The initial attempt to pass two Cargo test filters in one command was rejected
by Cargo's CLI parser. No test ran in that attempt; the two legal filtered
commands were run separately and passed.

## Concurrency reconciliation

T-046-01-02 committed its production work while the non-overlapping resolver
unit was being built:

- `900d9f3` added structured unsupported doctor results;
- `8f39362` preserved detailed failures through loop preflight;
- `d7bfe1e` and `e0181ee` added black-box stub tests.

Re-read both shared source files after those commits.

Preserved the unsupported-result display and detailed dependency propagation.

Removed Zellij only from the generic PATH-based check list because the dedicated
resolver now performs selection and version enforcement once.

Agent and WASM checks continue through the existing dependency machinery.

## Implementation update 2 — doctor and loop

Doctor now resolves the configured runtime and reports mode, version, supported
range, and absolute path in its required Zellij entry.

Loop now resolves the runtime before plugin/layout side effects, prints its
identity, and passes the exact path to both platform launch implementations.

Added a pure command-construction seam and assertions for managed and pinned
absolute programs.

Focused results:

- doctor tests: 44 passed, 0 failed;
- loop tests: 22 passed, 0 failed;
- runtime tests after integration: 9 passed, 0 failed.

## Planned test-fixture adaptation

The adjacent black-box fixture intentionally wrote no `[runtime]` section.

Under this ticket's required default that selects managed mode, so its PATH
stubs are correctly ignored and all four old system-mode assertions fail at the
missing managed-runtime boundary.

This is an expected semantic change, not a resolver defect.

After T-046-01-02 completes Review, update that shared fixture to explicitly set
`[runtime] zellij = "system"` and change its remedy assertion from prebuilt
binaries to Lisa's managed runtime.

## Fixture ownership resolution

T-046-01-02 retained ownership of its black-box test and committed the runtime-
aware adaptation as `79a2888`.

This ticket did not edit or include that test path.

The adapted built-CLI suite passed all four cases against this implementation.

## Source commits

Committed the resolver and config unit through Lisa's isolated transaction:

```text
8fd4781b4c9119bb1998cf9a134d36d3c53fc67a
Add configurable Zellij runtime resolver
```

Exact includes:

- `crates/lisa-cli/src/runtime.rs`;
- `crates/lisa-cli/src/config.rs`;
- `crates/lisa-cli/src/main.rs`.

Committed doctor and loop integration through Lisa's isolated transaction:

```text
c67f2355a64e0694dc904aec36b746ec282b32ce
Use resolved Zellij runtime in doctor and loop
```

Exact includes:

- `crates/lisa-cli/src/doctor.rs`;
- `crates/lisa-cli/src/loop_cmd.rs`.

No ordinary `git add` or `git commit` was used.

## Final verification

`cargo fmt --all -- --check` passed.

`git diff --check` passed.

`cargo test -p lisa-cli` passed:

- 14 CLI library unit tests;
- 295 CLI binary unit tests;
- all 13 executed black-box integration tests;
- one live real-Zellij harness intentionally ignored;
- zero failures.

`just check` passed:

- `cargo check -p lisa-plugin --target wasm32-wasip1` passed;
- workspace CLI library tests: 19 passed;
- workspace CLI binary tests: 295 passed;
- lisa-core unit tests: 207 passed;
- lisa-plugin unit tests: 395 passed;
- core and CLI integration tests passed;
- one declared live real-Zellij harness remained ignored;
- zero executed test failures.

## Final audit

Search found no `Command::new("zellij")` launch site in CLI source.

The only loop launch constructor is `Command::new(zellij_path)`.

Managed version is declared once in production code and used to build the path.

All five ticket-owned source paths are clean.

The ordinary Git index is empty.

Unrelated Lisa bookkeeping and planning paths remain untouched.

Implementation is complete; Review is next.
