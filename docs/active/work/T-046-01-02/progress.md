# Progress — T-046-01-02 doctor and loop floor enforcement

## Completed phase work

- Read `CLAUDE.md`, `AGENTS.md`, the ticket, and the complete RDSPI workflow.
- Mapped the shared T-046-01-01 version contract and both CLI consumers.
- Wrote `research.md` in the private attempt work directory.
- Evaluated four implementation approaches and chose a structured unsupported
  dependency result.
- Wrote `design.md`, `structure.md`, and `plan.md` in order.

## Implementation unit 1 — doctor classification and reporting

Completed in `crates/lisa-cli/src/doctor.rs`.

- Imported the shared classifier, verdict, and supported-range constant.
- Added one temporary remedy constant pointing at Zellij prebuilt static
  binaries on the releases page.
- Added an `Unsupported` dependency result distinct from `NotFound`.
- Classified the complete stdout from `zellij --version` rather than an opaque
  first-line string.
- Rendered detected version versus supported range for successful checks.
- Rendered canonical below-floor version, supported range, and remedy for
  incompatible checks.
- Rendered raw debug-quoted output in a separately named unparseable failure.
- Made required unsupported results fatal for doctor and loop-facing checks.
- Generalized misleading missing-only summary text.
- Preserved full rendered failures through `check_required_deps`.
- Added focused tests for 0.43.x, 0.44.x, 0.40.1, unparseable output, report
  formatting, fatality, and error-detail preservation.

Focused verification:

```text
cargo test -p lisa-cli doctor::tests
```

Result: 43 passed, 0 failed.

Committed through Lisa's isolated transaction:

```text
900d9f3ff3b09c0de4676f08bd403167d68fce96
Enforce Zellij support in doctor checks
```

Exact included path:

```text
crates/lisa-cli/src/doctor.rs
```

## Implementation unit 2 — loop refusal propagation

Completed in `crates/lisa-cli/src/loop_cmd.rs`.

- Added a pure dependency-preflight error formatter.
- Changed the loop heading from missing-only language to dependency-preflight
  failure language.
- Joined fully rendered failure reports without dropping diagnostic fields.
- Retained the `lisa doctor` follow-up guidance.
- Kept dependency checking before all WASM, cache, permission, layout, and exec
  side effects.
- Added a focused test proving 0.40.1, >= 0.43.0, remedy wording, and doctor
  guidance all survive into the returned loop error.

Focused verification:

```text
cargo test -p lisa-cli loop_cmd::tests::test_format_dependency_preflight_error_preserves_zellij_details
```

Result: 1 passed, 0 failed.

Package verification:

```text
cargo test -p lisa-cli
```

Result:

- 14 library unit tests passed;
- 281 binary unit tests passed;
- 13 black-box integration tests passed;
- 1 real-Zellij live harness remained intentionally ignored;
- 0 failures.

Committed through Lisa's isolated transaction:

```text
8f393626c1768b71ab7859ae6dadff813d6f2b8d
Surface Zellij incompatibility in loop preflight
```

Exact included path:

```text
crates/lisa-cli/src/loop_cmd.rs
```

## Full verification so far

```text
cargo fmt --all -- --check
```

Result: passed.

```text
cargo test --workspace
```

Result: passed across `lisa-core`, `lisa-cli`, and `lisa-plugin`, with no test
failures.

## Plan adjustment before final Review

The original plan relied on pure classifier tests and loop-detail propagation
tests because a clean development CLI can embed an empty WASM placeholder.

The acceptance criterion explicitly describes a stubbed `zellij` executable.

Before Review, add one focused Unix black-box integration test file that
constructs a temporary initialized Git project and prepends stubbed `zellij`
and `claude` executables to the child process PATH.

The test will prove:

- a 0.40.1 stub makes real `lisa loop` exit nonzero and report detected version,
  supported range, and static-binary remedy;
- 0.43.x and 0.44.x stubs pass the real dependency-preflight boundary;
- an unparseable stub makes real `lisa doctor` exit nonzero with its distinct
  unparseable unsupported message.

For supported versions, a clean build with empty embedded WASM may stop at the
existing development-only WASM guard after dependency preflight. The test will
accept that named later guard as evidence that the Zellij check passed; when a
release WASM is present, the launch stub exits successfully.

This is an additional source/test unit, not a change in production design.

It will be committed separately through `lisa commit-ticket` with only the new
integration-test path.

## Implementation unit 3 — stubbed process acceptance tests

Added `crates/lisa-cli/tests/zellij_version_preflight.rs`.

- Builds an isolated temporary Git project for each child CLI invocation.
- Prepends executable Zellij and Claude stubs to the child PATH without
  mutating the test process environment.
- Proves real `lisa loop` refuses `zellij 0.40.1` with the detected version,
  supported range, static-binary wording, and releases URL.
- Proves `zellij 0.43.9` and `zellij 0.44.3` pass real loop dependency
  preflight.
- Handles both release builds with embedded WASM and clean development builds
  that advance to the later named empty-WASM guard.
- Proves real `lisa doctor` rejects `zellij mystery-version` with a distinct
  unparseable unsupported message, supported range, and remedy.

Focused verification:

```text
cargo test -p lisa-cli --test zellij_version_preflight -- --nocapture
```

Result: 3 passed, 0 failed.

Committed through Lisa's isolated transaction:

```text
d7bfe1e8dfb4efc739bebde98fa721708d165c2e
Test stubbed Zellij version preflight
```

Exact included path:

```text
crates/lisa-cli/tests/zellij_version_preflight.rs
```

During Review, pass-case doctor output was identified as worth exercising at
the built-CLI boundary in addition to its unit coverage.

Added a fourth black-box test proving a supported 0.44.3 stub exits doctor
successfully and prints `detected 0.44.3, supported >= 0.43.0` with `OK`.

Re-ran the focused integration binary.

Result: 4 passed, 0 failed.

Committed the review improvement through Lisa's isolated transaction:

```text
e0181ee8e69b1fba95b1568fe1b1cc1fbc201107
Test supported Zellij doctor output
```

Exact included path remained:

```text
crates/lisa-cli/tests/zellij_version_preflight.rs
```

## Concurrency note

While the ticket was running, separate managed-runtime work appeared in
`crates/lisa-cli/src/config.rs`, `crates/lisa-cli/src/main.rs`, and the untracked
`crates/lisa-cli/src/runtime.rs`.

Those files belong to another ticket and were neither edited nor included by
T-046-01-02.

The black-box tests compiled and passed in the presence of that concurrent
work.

## Remaining work

- Re-run final package and formatting verification.
- Inspect combined commits and final source ownership.
- Write `review.md` and `review-disposition.json`.
