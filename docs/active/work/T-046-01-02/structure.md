# Structure — T-046-01-02 doctor and loop floor enforcement

## Change set

The implementation modifies two existing Rust source files.

No production file is created or deleted.

No manifest or lockfile change is needed.

The phase artifacts are created only in the current attempt work directory.

## `crates/lisa-cli/src/doctor.rs`

This file remains the owner of external dependency process checks, result
classification for CLI purposes, report formatting, and loop-facing dependency
preflight.

It receives all Zellij policy-consumer changes.

### Imports

Add imports from `lisa_core::version` for:

- `classify_zellij_version_output`;
- `ZellijVersionVerdict`;
- `SUPPORTED_ZELLIJ_RANGE`.

Keep the existing `AgentClient` import.

No semver dependency or direct semantic-version parsing enters the CLI.

### Remedy constant

Add a private `ZELLIJ_INSTALL_REMEDY: &str` near the dependency-check types.

Its value explicitly points users to Zellij prebuilt static binaries on the
official GitHub releases page.

Both absent and incompatible Zellij paths reuse this constant.

S-046-02 can replace this one UI constant when managed runtime installation is
available.

### `CheckResult`

Extend the private enum with:

```text
Unsupported {
    description: String,
    remedy: String,
}
```

`description` owns detected-versus-supported diagnostic prose.

`remedy` owns the actionable next step.

The existing variants remain unchanged for all non-Zellij checks.

### `CheckReport` display

Add one exhaustive match arm for `Unsupported`.

The first line shows the dependency name and the state `unsupported`.

The next indented line prints the description.

The final indented line prints `Remedy: <remedy>`.

The `Found`, `NotFound`, and `Skipped` rendering stays intact.

### Existing generic command helper

Leave `get_command_version` structurally unchanged.

It continues to return the first stdout line for Claude and Codex version
display.

Zellij stops using this helper because its complete output must be classified.

### Pure Zellij adapter

Add a private function with the conceptual signature:

```text
fn check_zellij_version_output(output: &str) -> CheckResult
```

The function matches the shared core verdict.

`InRange(version)` produces `Found` with a string in the form:

```text
detected <version>, supported <range>
```

`BelowFloor(version)` produces `Unsupported` with a description in the form:

```text
detected Zellij <version>; supported range <range>
```

`Unparseable` produces `Unsupported` with a description in the form:

```text
unparseable Zellij version output <debug-quoted trimmed output>;
supported range <range>
```

Both unsupported branches copy the remedy constant into the result.

The helper contains no process or filesystem operations.

### `check_zellij`

Replace its use of `get_command_version` with direct command execution.

The command remains `zellij --version`.

On a successful process status, pass the complete lossily decoded stdout to
`check_zellij_version_output`.

On spawn failure or nonzero status, return `NotFound` with the remedy constant
as its install hint.

The function remains a zero-argument closure target for `build_checks`.

### Failure predicate

Update `has_failures` to match required reports whose result is either:

- `NotFound`;
- `Unsupported`.

Do not make `Skipped` fatal.

Do not change the `required` field behavior.

### Report summary

Change `format_report`'s failure footer to say required dependencies are
unavailable or unsupported.

Keep the success footer unchanged.

### Loop-facing aggregation

Update the comment on `check_required_deps` to say errors contain rendered
failure details rather than missing dependency names.

Keep its signature `Result<(), Vec<String>>`.

In `check_required_deps_inner`, filter required reports using the same absent or
unsupported classification.

Map each failing report to `report.to_string()` rather than `report.name`.

The resulting vector preserves name, detected value, supported range, and
remedy for the loop caller.

### Doctor terminal error

Change `run_doctor`'s returned error to the generalized unavailable-or-
unsupported wording.

The complete printed report continues to precede the returned error.

No changes are needed in `main.rs`; it already prints the error and exits 1.

### Doctor test helpers

Add `mock_unsupported(name, description, remedy) -> DependencyCheck` next to
the found, not-found, and skipped helpers.

It returns a required dependency by default, matching the other fatal fixtures.

### Zellij classification tests

Add a test for 0.43.x output.

It asserts the result is `Found` and its formatted value includes the detected
version and `SUPPORTED_ZELLIJ_RANGE`.

Add a test for 0.44.x output with the same in-range expectations.

Add a test for 0.40.1.

It asserts `Unsupported` and formatted output contains:

- `0.40.1`;
- the shared range string;
- `unsupported`;
- `prebuilt static binaries`;
- the releases URL.

Add a test for invalid output.

It asserts `Unsupported` and formatted output contains:

- the invalid raw string;
- `unparseable Zellij version output`;
- the shared range;
- the static-binary remedy.

### Generic report tests

Add an unsupported report to the failure-format test or a dedicated test.

Assert the generalized report footer is used.

Update the existing missing-only footer expectation.

Add `has_failures` coverage for a required unsupported dependency.

Add direct display coverage for `CheckResult::Unsupported`.

### Required-dependency tests

Keep all-found and optional-skipped success cases.

Update one-missing and all-missing assertions because errors now contain full
rendered report strings rather than bare names.

Prefer content assertions over exact whitespace snapshots for alignment
resilience.

Add a required-unsupported aggregation test.

Assert the returned entry contains the dependency name, incompatibility detail,
and remedy.

## `crates/lisa-cli/src/loop_cmd.rs`

This file remains the owner of loop startup sequencing and terminal preflight
error context.

It does not parse versions directly.

### Error formatter

Add a small private pure function near `validate_project_protocol` with the
conceptual signature:

```text
fn format_dependency_preflight_error(
    client: AgentClient,
    failures: &[String],
) -> String
```

The heading is:

```text
Dependency preflight failed for <client>:
```

The body joins rendered failures with newlines.

The footer retains the instruction to run `lisa doctor` for the complete
dependency report and install guidance.

The function does not know about Zellij fields, constants, or verdict types.

### `run_loop`

Keep the current placement of dependency checks.

Replace the inline `map_err` string construction with a call to the formatter.

Rename the closure binding from `missing` to `failures` to reflect its expanded
meaning.

No other startup ordering changes.

In particular, failure still occurs before:

- Codex trust writes;
- embedded-WASM validation;
- temporary WASM writes;
- cache cleanup;
- permission pre-grants;
- layout writes;
- Zellij process replacement.

### Loop tests

Add a pure unit test for `format_dependency_preflight_error`.

Supply one representative rendered failure containing detected Zellij 0.40.1,
supported range >= 0.43.0, and the static-binary remedy.

Assert the final error contains:

- the preflight-failed heading;
- the client name;
- 0.40.1;
- >= 0.43.0;
- prebuilt static binaries;
- `lisa doctor`.

This test locks the formerly lossy boundary between doctor checks and loop
refusal.

Existing dry-run and layout tests remain unchanged.

## Artifact files

`research.md`, `design.md`, `structure.md`, `plan.md`, `progress.md`, and
`review.md` are written under the private attempt work directory.

`review-disposition.json` is written there during Review.

These artifacts are not included in ticket source commits because Lisa owns
their admission and final publication.

## Commit boundaries

The first meaningful source unit is doctor classification and reporting.

Commit it with exact include path:

```text
crates/lisa-cli/src/doctor.rs
```

The second meaningful source unit is loop refusal propagation.

Commit it with exact include path:

```text
crates/lisa-cli/src/loop_cmd.rs
```

No ordinary-index operation is used.

Before Review, both source paths must be clean relative to HEAD and no other
ticket-owned source path may remain untracked or modified.
