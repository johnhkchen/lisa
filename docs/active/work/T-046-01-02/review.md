# Review — T-046-01-02 doctor and loop floor enforcement

## Disposition

Ready to complete.

The shared Zellij range check is enforced at both required CLI boundaries.

Below-floor and unparseable system Zellij outputs fail closed.

Successful doctor output reports detected version versus supported range.

Loop failures retain the version-policy diagnostic and remedy rather than
collapsing it to a dependency name.

## Source changes

### `crates/lisa-cli/src/doctor.rs`

Added a first-class `Unsupported` dependency result.

This distinguishes an installed incompatible tool from a tool that cannot be
found or executed.

Added display formatting for the unsupported state with a description and an
explicit remedy line.

Wired `lisa_core::version::classify_zellij_version_output` and
`SUPPORTED_ZELLIJ_RANGE` into the CLI dependency check.

The original implementation executes `zellij --version` and classifies the
complete stdout rather than truncating to the first line.

Supported versions render in the form `detected <version>, supported >=
0.43.0` and retain the generic `OK` marker.

Below-floor versions render the canonical detected version and shared range.

Unparseable versions render a distinct message with debug-quoted raw output.

The temporary remedy points at Zellij prebuilt static binaries on the official
releases page.

Required unsupported dependencies now participate in doctor failure detection.

Doctor's summary and returned error no longer incorrectly describe every
failure as missing.

`check_required_deps` now preserves full rendered failure details instead of
returning only dependency names.

The file includes unit coverage for supported 0.43 and 0.44 versions, 0.40.1,
unparseable output, display formatting, fatality, and detailed aggregation.

### `crates/lisa-cli/src/loop_cmd.rs`

Added a pure dependency-preflight error formatter.

Loop startup now carries the complete doctor failure report into its terminal
error.

The heading uses general preflight-failure language rather than missing-only
language.

The `lisa doctor` follow-up remains present.

The check stays before WASM validation and all launch side effects.

The formatter test proves that the detected 0.40.1 value, >= 0.43.0 range,
runtime remedy, and doctor guidance survive the call boundary.

### `crates/lisa-cli/tests/zellij_version_preflight.rs`

Added a Unix black-box test binary around `CARGO_BIN_EXE_lisa`.

Each invocation builds an isolated temporary Git project.

It prepends executable Zellij and Claude stubs to only the child process PATH.

It does not mutate the test runner's global environment.

The fixture explicitly configures `[runtime] zellij = "system"`.

That keeps the system-PATH acceptance boundary stable after the adjacent
managed-runtime story makes managed mode the default.

The 0.40.1 loop case asserts a nonzero exit, detected version, shared floor,
and an actionable runtime remedy.

The supported loop case checks both 0.43.9 and 0.44.3.

In release-like builds it accepts the successful stub exec.

In clean development builds it accepts only the later named empty-WASM guard,
which demonstrates that dependency preflight admitted the supported version.

The doctor pass case asserts a successful exit, 0.44.3, the supported range,
and `OK`.

The doctor malformed-output case asserts a nonzero exit, `unsupported`, the
bad output, the floor, and a remedy.

The remedy assertion recognizes the intended lifecycle in the story:

- prebuilt static binaries before managed runtime exists;
- the managed runtime once S-046-02 is integrated.

## Commit review

Ticket-owned source and test work was committed only through Lisa's isolated
transaction.

Commit `900d9f3ff3b09c0de4676f08bd403167d68fce96`:

```text
Enforce Zellij support in doctor checks
```

Exact included path:

```text
crates/lisa-cli/src/doctor.rs
```

Commit `8f393626c1768b71ab7859ae6dadff813d6f2b8d`:

```text
Surface Zellij incompatibility in loop preflight
```

Exact included path:

```text
crates/lisa-cli/src/loop_cmd.rs
```

Commit `d7bfe1e8dfb4efc739bebde98fa721708d165c2e`:

```text
Test stubbed Zellij version preflight
```

Exact included path:

```text
crates/lisa-cli/tests/zellij_version_preflight.rs
```

Commit `e0181ee8e69b1fba95b1568fe1b1cc1fbc201107` added the black-box successful
doctor report assertion in the same exact test path.

Commit `79a2888a2479aff1446942aac6662245348bd0cc` made the same fixture explicitly
system-mode and compatible with the adjacent managed-runtime integration.

`git show --check` passed for the implementation commits reviewed before the
final small test adaptation.

No ordinary `git add`, ordinary `git commit`, or broad index operation was
used.

The ordinary index was empty during final ownership checks.

## Acceptance review

### Below-floor loop refusal

Met.

The real child-process test supplies `zellij 0.40.1` from the first PATH entry.

`lisa loop` exits nonzero.

The diagnostic includes `Zellij 0.40.1` and `>= 0.43.0`.

It also contains the applicable remedy: prebuilt static binaries in the base
implementation, or managed runtime once the adjacent runtime resolver exists.

### Supported loop versions

Met.

The same real preflight accepts stub outputs `zellij 0.43.9` and `zellij
0.44.3`.

The pure doctor adapter also has separate 0.43.x and 0.44.x unit cases.

### Doctor pass report

Met.

Both unit and black-box coverage verify the detected 0.44.3 version, supported
range >= 0.43.0, and successful status.

The adjacent resolver enriches this report with mode and exact executable path
without removing version-versus-range information.

### Doctor below-floor and unparseable failures

Met.

The below-floor classification is independently unit-tested and exercised by
the real loop boundary.

The unparseable doctor test uses `zellij mystery-version` and verifies its own
named unsupported diagnostic rather than success or generic absence.

## Test coverage

Focused doctor unit suite:

```text
cargo test -p lisa-cli doctor::tests
```

Result: 43 passed, 0 failed.

Focused loop propagation test:

```text
cargo test -p lisa-cli loop_cmd::tests::test_format_dependency_preflight_error_preserves_zellij_details
```

Result: 1 passed, 0 failed.

Final stubbed-process suite:

```text
cargo test -p lisa-cli --test zellij_version_preflight
```

Result: 4 passed, 0 failed against the current adjacent runtime integration.

CLI package suite was run after the production implementation and initial
black-box coverage.

Its latest full run included:

- 14 library unit tests passed;
- 293 binary unit tests passed, including concurrent runtime-resolver tests;
- all ordinary black-box tests passed;
- the existing real-Zellij harness remained ignored;
- 0 failures.

Workspace suite:

```text
cargo test --workspace
```

Result: passed across the CLI, core, and plugin crates with no failures.

Formatting verification:

```text
cargo fmt --all -- --check
```

Result: passed.

## Concurrency and ownership

T-046-02-01 began managed-runtime work concurrently and intentionally rebased
its `doctor.rs` and `loop_cmd.rs` integration on this ticket's committed API.

Its progress artifact explicitly identifies those shared files and this
ticket's active overlap.

Current uncommitted changes in those two files, plus `config.rs`, `main.rs`, and
`runtime.rs`, belong to T-046-02-01 and were not included in this ticket's
commits.

This ticket's own deltas in `doctor.rs`, `loop_cmd.rs`, and the integration test
are durable in the five listed Lisa commits.

The integration test was rerun successfully against the concurrent resolver
changes, providing evidence that the two tickets compose.

Unrelated Lisa bookkeeping, planning documents, and other story files remain
outside this ticket's ownership.

## Open concerns and limitations

The focused black-box test uses Unix shell stubs and is compiled only on Unix.

The underlying classification and report unit tests remain platform-neutral.

The real-Zellij delivery harness was not enabled because it requires the live
toolchain and WASM target contract; it is unrelated to the deterministic
version policy exercised here.

The base implementation's static-binary remedy is intentionally temporary.

The adjacent managed-runtime ticket already replaces it with managed-mode
guidance, as the ticket context requires.

No critical defect, missing acceptance behavior, or ticket-owned TODO remains.
