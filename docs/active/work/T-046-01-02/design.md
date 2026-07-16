# Design — T-046-01-02 doctor and loop floor enforcement

## Objective

Make Zellij compatibility a required CLI preflight rather than an opaque
presence check.

Reuse the `lisa-core` version parser and range constant as the only policy
source.

Keep successful and failed doctor output explicit about detected versus
supported versions.

Carry the same incompatibility detail into `lisa loop` so the launch refusal is
self-contained and actionable.

Fail closed for malformed successful output from `zellij --version`.

## Option 1 — parse only in `run_doctor`

`run_doctor` could inspect the existing opaque Zellij report after dependency
checks and append a warning or return an error.

Advantages:

- localized user-output change;
- little disturbance to generic dependency machinery;
- easy to append extra prose to the doctor report.

Disadvantages:

- loop calls `check_required_deps`, not `run_doctor`;
- the loop would retain the unsafe opaque-success behavior;
- parsing would occur after `check_zellij` already mislabeled the result;
- doctor and loop would likely grow separate formatting and policy paths.

Rejected because the shared dependency check is the natural enforcement point
for both commands.

## Option 2 — parse independently in doctor and loop

Each command could execute `zellij --version`, invoke the core classifier, and
format its own message.

Advantages:

- each command can tailor its exact wording;
- no changes to `CheckResult` or dependency aggregation;
- loop can place the check at any desired startup point.

Disadvantages:

- duplicates process execution and classification glue;
- diagnostics can drift between commands;
- mixed-provider loop checks become harder to reason about;
- the generic dependency report would still claim Zellij is found before a
  separate policy check rejects it;
- tests would need to cover two parallel translations of the same verdict.

Rejected because one host-policy adapter should feed both surfaces.

## Option 3 — encode unsupported as `NotFound`

`check_zellij` could return `NotFound` with an install hint whenever the
classifier reports below-floor or unparseable output.

Advantages:

- no new enum variant;
- existing failure predicates and loop aggregation already recognize it;
- remedy text fits the existing install-hint field.

Disadvantages:

- reports that an installed binary was not found;
- loses the semantic distinction required by the ticket;
- doctor formatting cannot naturally say detected-versus-supported;
- the summary remains misleading;
- project-version checks already overload `NotFound`, making it less desirable
  to add another distinct condition to that bucket.

Rejected because incompatible and absent are operationally different states.

## Option 4 — add a structured unsupported result

Extend private `CheckResult` with an `Unsupported` variant containing a
description and remedy.

Translate all three core verdicts inside one Zellij-specific adapter.

Advantages:

- both doctor and loop consume the same verdict translation;
- required-failure logic can explicitly recognize unsupported tools;
- the report distinguishes incompatibility from absence;
- description and remedy survive through generic aggregation;
- no public API or cross-crate type change is needed;
- mock checks can cover the new state without environment mutation.

Disadvantages:

- every exhaustive match on `CheckResult` must be updated;
- the generic report machinery now understands one more failure class;
- success still uses the generic version string field.

Chosen because it makes compatibility a first-class dependency state while
remaining inside the existing CLI boundary.

## Zellij command execution

Keep `get_command_version` unchanged for Claude and Codex.

Give `check_zellij` its own direct `Command::new("zellij")` execution.

Invoke `--version` and require a successful exit status.

On spawn failure or nonzero status, return `NotFound` with the static-binary
remedy.

On success, convert the entire stdout buffer with `String::from_utf8_lossy`.

Do not truncate at the first line.

Pass the complete string to a pure `check_zellij_version_output` helper.

The pure helper allows exact unit coverage for supported, below-floor, and
unparseable inputs without changing global PATH.

Using the complete stdout preserves the grammar established in `lisa-core` and
prevents a valid first line from hiding unexpected trailing output.

Stderr remains outside the version grammar because ordinary `--version`
contract data is stdout and the existing implementation already uses stdout.

## Verdict translation

Import `classify_zellij_version_output`, `ZellijVersionVerdict`, and
`SUPPORTED_ZELLIJ_RANGE` from `lisa_core::version`.

For `InRange(version)`, return `Found` with a display string containing:

- `detected <canonical version>`;
- `supported <range>`.

The generic `CheckReport` formatter appends `OK`.

For `BelowFloor(version)`, return `Unsupported` with a description containing:

- `detected Zellij <canonical version>`;
- `supported range <shared range>`.

For `Unparseable`, return `Unsupported` with a distinct description containing:

- the phrase `unparseable Zellij version output`;
- the trimmed raw stdout represented with debug quoting;
- `supported range <shared range>`.

Debug quoting makes empty strings, newlines, and control characters visible.

Both unsupported variants use one constant remedy string.

## Remedy wording

Declare a private CLI constant for the temporary remedy.

The wording will explicitly say `Zellij prebuilt static binaries` and link to
Zellij's GitHub releases page.

This is more specific than the current Cargo-install recommendation and aligns
with the story's cross-platform runtime strategy.

The constant is intentionally in the CLI check rather than `lisa-core`.

Runtime support policy is stable shared domain logic; installation guidance is
a user-interface concern that S-046-02 will later replace with the managed
runtime remedy.

The same constant serves missing, below-floor, and unparseable Zellij results.

## Report formatting

Render unsupported reports as a named `unsupported` state followed by an
indented description and `Remedy:` line.

Keep the existing `Found`, `NotFound`, and `Skipped` formats for other tools.

Generalize the failure summary from `Some dependencies are missing` to
`Some required dependencies are unavailable or unsupported`.

Generalize `run_doctor`'s returned error the same way.

This avoids falsely describing an installed 0.40.1 binary as absent.

Passing output remains compact even though the Zellij version field is longer
than the original alignment width.

The required facts matter more than fixed column alignment.

## Required-failure semantics

Update `has_failures` so a required `Unsupported` report is fatal alongside a
required `NotFound` report.

Optional `Skipped` remains non-fatal.

Optional project-version `NotFound` behavior remains as it is today.

Update `check_required_deps_inner` to collect rendered required failures rather
than only report names.

Its type can remain `Result<(), Vec<String>>`, limiting call-site changes.

For a missing agent, the vector element becomes its full existing report.

For an unsupported Zellij host, the element includes the detected version,
supported range, and remedy.

This is a private crate API used only by loop preflight and inline tests.

## Loop refusal

Keep external dependency checks in their current position after project and Git
validation but before all WASM, cache, permission, layout, and exec operations.

Change loop wording from `Missing dependencies` to `Dependency preflight
failed`.

Join rendered failures with newlines beneath that heading.

The resulting error for 0.40.1 is self-contained and contains all acceptance
facts even if the operator does not subsequently run doctor.

Retain the suggestion to run `lisa doctor` for the complete report.

Extract the loop error construction into a small pure formatter.

Unit-test that formatter with a representative Zellij incompatibility string
to lock the propagation boundary.

Dry-run behavior remains unchanged: it models DAG execution and does not require
installed runtime tools.

## Mixed-provider behavior

Do not broaden this ticket into dependency de-duplication.

The current loop invokes the complete check set once per configured provider,
which can execute Zellij twice for mixed Claude/Codex routing.

Both calls use the same policy and fail before launch.

Removing duplication would require splitting host dependencies from
provider-specific dependencies and carries unrelated regression risk.

A later refactor can address that without changing the user-visible contract.

## Testing strategy

Add doctor unit tests for pure translation of:

- `zellij 0.43.x` to an in-range found result;
- `zellij 0.44.x` to an in-range found result;
- `zellij 0.40.1` to a below-floor unsupported result;
- an invalid string to a distinct unparseable unsupported result.

Assert formatted pass output includes detected version, shared range, and `OK`.

Assert both unsupported outputs include the raw or canonical detected value,
the floor through shared range display, the word `unsupported`, and the static
binary remedy.

Add mock-unsupported coverage to `has_failures`, report summary formatting, and
required-dependency aggregation.

Update existing aggregation expectations from bare names to rendered reports.

Add a loop formatter unit test proving detailed failures survive into the final
preflight refusal.

Run focused package tests first, then the workspace suite and formatting check.

## Rejected scope

Do not alter the numeric support floor.

Do not change `lisa-core` parsing or add dependencies.

Do not install or download Zellij automatically.

Do not implement the S-046-02 managed runtime remedy early.

Do not require Zellij during `lisa loop --dry-run`.

Do not change validate's existing tool-presence semantics.

Do not modify plugin code or the Zellij SDK pin.
