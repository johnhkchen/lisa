# Research — T-046-01-02 doctor and loop floor enforcement

## Ticket boundary

T-046-01-02 consumes the Zellij version contract introduced by T-046-01-01.

The ticket changes host-side CLI behavior only.

It does not change the supported version floor, plugin SDK pin, plugin protocol,
managed-runtime installation, scheduler behavior, or ticket lifecycle.

The required floor is Zellij 0.43.0.

The supported range is open-ended and rendered by `lisa-core` as `>= 0.43.0`.

The user-visible requirement has two surfaces:

- `lisa doctor` must report detected version versus supported range;
- `lisa loop` must refuse unsupported or unparseable Zellij hosts before launch.

Both failure paths must name a remedy.

The remedy is a prebuilt static Zellij binary until story S-046-02 provides a
Lisa-managed runtime.

## Shared version contract

`crates/lisa-core/src/version.rs` is present in the current tree.

It defines the domain newtype `ZellijVersion` over `semver::Version`.

`ZellijVersion::parse_command_output` accepts exactly the product token
`zellij` followed by one semantic-version token.

It accepts normal surrounding and repeated whitespace.

It rejects empty output, missing versions, a different product, invalid
semantic versions, and extra tokens.

The type implements semantic ordering and canonical display.

`SUPPORTED_ZELLIJ_RANGE` owns the single floor value 0.43.0.

Its `Display` implementation produces `>= 0.43.0`.

`classify_zellij_version_output` returns one of three verdicts:

- `InRange(ZellijVersion)`;
- `BelowFloor(ZellijVersion)`;
- `Unparseable`.

Parsed verdicts retain the canonical detected version for diagnostics.

Unparseable verdicts do not retain the input, so a CLI diagnostic that names
the bad output must keep the original command stdout alongside classification.

The core tests already prove the exact floor, newer 0.43 and 0.44 releases,
0.40.1, prerelease ordering, garbage rejection, and canonical display.

This ticket should not duplicate semantic parsing or the numeric floor.

## Current doctor dependency model

`crates/lisa-cli/src/doctor.rs` owns dependency discovery and reporting.

`CheckResult` currently has three variants:

- `Found { version: String }`;
- `NotFound { install_hint: String }`;
- `Skipped { reason: String }`.

There is no state for a binary that exists but is incompatible.

`CheckReport` pairs a static dependency name, required flag, and result.

Its `Display` implementation formats successful versions with `OK`, absent
tools with an install hint, and optional skipped checks with a reason.

`DependencyCheck` stores a boxed closure returning `CheckResult`.

This closure boundary makes the report machinery straightforward to unit test
without invoking real binaries.

`run_checks` executes all supplied closures and collects reports.

`format_report` prints the dependency heading, every report, and one summary.

`has_failures` currently treats only required `NotFound` results as failures.

Any new incompatible state must be included in this predicate or doctor will
still exit successfully.

## Current process-output boundary

`get_command_version` launches a command with arguments and captures stdout.

It returns `None` on spawn failure or nonzero process status.

On success it converts stdout lossily to UTF-8.

It then keeps only the first stdout line and trims it.

That behavior is adequate for opaque version display but is weaker than the
shared Zellij output grammar.

If Zellij output contains a valid first line followed by extra text, truncating
to the first line would allow the shared parser to accept an output it would
otherwise reject.

The Zellij check therefore needs access to the complete stdout payload.

Claude and Codex checks can retain the existing first-line behavior because
this ticket adds no version policy for those clients.

Lossy UTF-8 conversion is an established boundary in this module.

An empty successful stdout currently becomes `Found` with an empty string.

For Zellij it must instead classify as unparseable and fail closed.

## Current Zellij doctor check

`check_zellij` calls `get_command_version("zellij", &["--version"])`.

Every successful command output becomes `CheckResult::Found` without parsing.

The current successful line therefore echoes an opaque string such as
`zellij 0.43.1`.

The current missing-tool hint recommends `cargo install zellij`, followed by
the generic Zellij installation documentation.

The ticket's remedy requirement instead points at prebuilt static binaries.

The doctor check is part of `build_checks` for both Claude and Codex projects.

The optional WASM target check is unrelated and remains non-fatal when skipped.

## Current doctor command path

`run_doctor` loads the configured agent client, defaulting to Claude when the
project configuration cannot be loaded.

It runs dependency checks, appends the project-version check, performs cache
cleanup, and optionally seeds Codex trust.

It prints the assembled report before returning an error when required checks
failed.

`main.rs` renders that returned error on stderr and exits with status 1.

The report itself is therefore visible even on a failing doctor invocation.

Project-version failures reuse `NotFound` despite representing stale config,
but they are marked optional and do not affect the required-dependency result.

Changing the generic meaning of `NotFound` would risk unrelated project checks.

A distinct unsupported dependency result fits the existing boundary better.

## Current loop preflight

`crates/lisa-cli/src/loop_cmd.rs` validates project structure and project
protocol before checking external dependencies.

Dry-run mode returns before checking external dependencies by design.

Real loop mode discovers the Git root and scans tickets to determine every
agent provider the DAG can route to.

For each configured provider it calls `doctor::check_required_deps`.

That function builds the complete dependency list, including Zellij, the
selected agent binary, and the optional WASM target.

Mixed-provider loops consequently run the Zellij check more than once.

The duplication already exists and is outside the ticket's core behavior.

`check_required_deps_inner` currently returns `Err(Vec<String>)` containing
only the names of required missing dependencies.

`run_loop` formats those names as `Missing dependencies for <client>` and
directs the user to `lisa doctor` for details.

This lossy return shape cannot satisfy the acceptance criterion because a loop
refusal must itself name the detected version, floor, and remedy.

The error boundary must preserve a formatted failure detail or an equivalent
structured result through to `run_loop`.

The later startup path writes the embedded WASM, cleans caches, pre-grants
permissions, writes `.lisa-layout.kdl`, and replaces the process with Zellij.

Version refusal must occur before those launch side effects.

Dependency checking already precedes all of those operations.

## Existing unit-test conventions

`doctor.rs` has inline unit tests for report execution, formatting, failure
detection, required dependency aggregation, client selection, cache routing,
trust seeding, and project-version reporting.

Test helpers create mock dependency closures for found, absent, and skipped
results.

These helpers are the natural place to add an unsupported-result fixture.

Focused pure tests can classify supplied Zellij stdout without mutating PATH.

Report tests can prove both successful detected-versus-range text and each
unsupported diagnostic shape.

Required-dependency tests can prove that unsupported results propagate a full
diagnostic rather than only the dependency name.

`loop_cmd.rs` has inline tests for project preconditions, protocol refusal,
dry-run DAG behavior, client discovery, layout generation, and Git discovery.

Its current `run_loop` unit tests use dry-run when they need to avoid external
dependencies and process replacement.

## Existing integration-test conventions

`crates/lisa-cli/tests` contains black-box tests that invoke
`CARGO_BIN_EXE_lisa` with `std::process::Command`.

Shell fixtures elsewhere in the crate create executable Zellij wrappers that
respond specially to `--version` and to launch arguments.

Those live harnesses are much broader than the version-policy ticket.

A focused CLI integration test could construct a temporary project and prepend
a stub directory to PATH.

However, a successful real loop also requires an embedded nonempty WASM.

`crates/lisa-cli/build.rs` embeds the release plugin only when the target WASM
already exists; otherwise ordinary development builds embed an empty
placeholder.

Therefore a clean `cargo test -p lisa-cli` cannot reliably expect a supported
normal loop invocation to reach and successfully exec the Zellij stub.

Pure doctor tests plus loop propagation tests avoid tying policy coverage to a
pre-existing release artifact.

Hidden or release-path black-box checks can still exercise the real stubbed
binary because production preflight uses the same process check.

## Diagnostic constraints

The passing doctor line must contain the canonical detected version and the
shared supported-range display.

The below-floor line must distinguish an existing incompatible binary from a
missing binary.

It must contain 0.40.1, the 0.43.0 floor through the shared range, and the
prebuilt-static-binary remedy.

The unparseable line must explicitly call the output unparseable or otherwise
name version-output parsing as the unsupported condition.

It should quote or escape raw output so blank, multiline, or unusual data does
not produce an ambiguous report.

The same detail must survive into the loop's returned error.

Generic missing-agent behavior should remain recognizable and actionable.

The doctor summary currently says all failures are missing; that becomes
inaccurate once unsupported binaries are represented and should be generalized.

## Repository and ownership constraints

The worktree contains Lisa bookkeeping changes and many unrelated untracked
epic, story, ticket, and knowledge files.

Those paths are outside this ticket and must remain untouched.

T-046-01-02 is expected to own changes in `crates/lisa-cli/src/doctor.rs` and
`crates/lisa-cli/src/loop_cmd.rs`.

No `lisa-core` source change is required because T-046-01-01 already exposes
the needed contract.

No dependency or lockfile change is expected.

Attempt artifacts belong only under
`.lisa/attempts/T-046-01-02/1/work/` until Lisa publishes them.

Implementation commits must use `lisa commit-ticket` with exact repository
paths and must not consume unrelated ordinary-index or worktree state.
