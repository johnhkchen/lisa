# Structure — T-046-04-02

## Change set overview

The implementation modifies three existing Rust files.

No production module is added or removed.

No public CLI argument or library API changes.

The existing doctor dependency-report model remains the architectural center.

The loop continues consuming doctor-rendered dependency failures.

## `crates/lisa-cli/src/doctor.rs`

This file owns the new required checks and the released-installer remediation
constant.

### Imports

Add an import for `crate::templates::PLUGIN_WASM`.

Keep existing configuration and runtime imports unchanged.

### Constants

Add a `pub(crate)` string constant containing the complete Lisa shell-installer
command.

The visibility permits `loop_cmd.rs` to use exactly the same supported remedy.

The constant remains internal to the binary crate.

The existing test-only Zellij static-binary constant remains test-only.

### Git checker

Add private `fn check_git() -> CheckResult` near the Claude and Codex checkers.

It calls `get_command_version("git", &["--version"])`.

On success it returns `CheckResult::Found { version }`.

On failure it returns `CheckResult::NotFound` with
`sudo apt install git` as the install hint.

It does not introduce a platform detector or package-manager abstraction.

### Embedded-WASM checker

Add private `fn check_embedded_wasm_bytes(wasm: &[u8]) -> CheckResult`.

This function contains the actual classification and is directly unit-testable.

Nonempty input returns `Found` with an embedded detail.

Empty input returns `Unsupported` with a description naming the empty
placeholder and a remedy using the shell-installer constant.

Add private `fn check_embedded_wasm() -> CheckResult`.

It delegates to `check_embedded_wasm_bytes(PLUGIN_WASM)`.

This wrapper is the closure target used by real dependency construction.

### Dependency composition

Extend `build_checks(client)` with a required `git` entry.

Extend it with a required `embedded WASM` entry.

Retain the selected agent as required.

Retain `wasm target` as optional.

The vector ordering will put user runtime requirements before the optional
developer diagnostic.

The exact order is:

1. Git.
2. Selected agent client.
3. Embedded WASM.
4. Optional Rust WASM target.

Zellij remains outside this vector because doctor reports its resolved runtime
separately.

### Unit tests

Replace generic fixtures containing the prohibited Zellij Cargo command.

Use a managed-runtime remedy in Zellij-specific formatting tests.

Use neutral sample install text for generic missing-dependency tests.

Add a test that formats the real Zellij runtime failure and asserts its remedy
contains `managed`.

Add empty-WASM classification/report tests.

The failure test asserts:

- the check is unsupported;
- the description names an empty embedded WASM placeholder;
- the remedy includes the released installer asset;
- the remedy does not include source-build commands.

The success test supplies nonempty bytes and asserts `Found`.

Update `build_checks` composition tests to expect `git` and `embedded WASM` for
both selected clients.

Keep assertions that only the selected agent is included.

Add a dependency-report mock test for the exact Git apt remedy if not already
covered by the integration boundary.

## `crates/lisa-cli/src/loop_cmd.rs`

This file owns real-loop ordering and the final defensive empty-WASM guard.

### Error helper

Add private `fn embedded_wasm_error() -> String` near other preflight formatters.

The message begins with the existing recognizable phrase that the WASM plugin
is not embedded.

It names the empty placeholder condition.

It tells the user to reinstall Lisa with the shell installer.

It interpolates `doctor::LISA_SHELL_INSTALL_COMMAND`.

It contains no `cargo install`, `git clone`, or `just release` path.

### Real-loop ordering

Remove the early `discover_git_root(root)?` immediately after dry-run handling.

Leave Zellij runtime resolution in place.

Leave configured-client discovery and dependency preflight in place.

Insert `discover_git_root(root)?` after all client dependency preflights
succeed.

The variable remains available before Codex trust, plugin output, and layout
generation.

No dry-run ordering changes.

### Empty-WASM guard

Replace the inline multiline source-build string with
`return Err(embedded_wasm_error())`.

Keep the guard at its current location before plugin bytes are hashed or
written.

### Unit tests

Add a test for `embedded_wasm_error()`.

Assert it contains `WASM plugin not embedded`.

Assert it contains the complete shell-installer asset name or command.

Assert it does not contain `git clone`.

Assert it does not contain `just release`.

The test is placed in the existing internal test module alongside preflight
formatter tests.

Update any fixed dependency fixture that still embeds old Zellij wording.

## `crates/lisa-cli/tests/zellij_version_preflight.rs`

This Unix integration test file owns process-level doctor behavior under a
controlled PATH.

### Harness extension

Retain `run_with_zellij_version` as the default helper for existing tests.

Factor or add a lower-level helper that accepts whether the host PATH should be
appended.

Project setup, `.lisa.toml`, Git repository initialization, and Zellij/Claude
stub creation remain shared.

Normal tests continue appending the host PATH so their behavior does not drift.

The missing-Git test invokes the CLI with only the temporary stub directory on
PATH.

That directory contains Zellij and Claude, but deliberately no Git.

### Git absence test

Add `doctor_names_missing_git_and_apt_remedy`.

Run `doctor` with a supported system Zellij stub and isolated PATH.

Assert the process exits unsuccessfully.

Assert stdout contains `git`.

Assert stdout contains `not found`.

Assert stdout contains `sudo apt install git`.

The test reads stdout because doctor prints its structured report before main
prints the final error to stderr.

### Existing placeholder expectations

Existing loop tests that allow `WASM plugin not embedded` remain compatible
because the message prefix is preserved.

Doctor success tests may need to account for whether the Cargo-built test binary
contains a plugin.

The integration suite should not assert doctor success if the new required
embedded-WASM check correctly detects an empty test placeholder.

Where current tests need to inspect successful Zellij reporting, they should
accept an expected empty-WASM failure after confirming the Zellij row itself is
successful.

## Attempt artifacts

The following files are written only under
`.lisa/attempts/T-046-04-02/1/work/`:

- `research.md`;
- `design.md`;
- `structure.md`;
- `plan.md`;
- `progress.md`;
- `review.md`;
- `review-disposition.json`.

They are not passed to the ticket source commit.

Lisa publishes admitted artifacts later.

## Ownership and commit boundary

The ticket-owned source unit consists exactly of:

- `crates/lisa-cli/src/doctor.rs`;
- `crates/lisa-cli/src/loop_cmd.rs`;
- `crates/lisa-cli/tests/zellij_version_preflight.rs`.

All three paths implement one cohesive preflight/remediation behavior change.

They will be committed together through `lisa commit-ticket` after focused and
workspace verification.

No unrelated metadata, active tickets, stories, epics, or historical documents
will be included.

## Resulting control flow

Doctor loads configuration.

Doctor resolves Zellij.

Doctor evaluates Git, selected agent, embedded WASM, and optional Rust target.

Doctor renders all dependency rows.

Doctor returns failure when Git or the embed is unavailable.

Real loop validates local project structure and protocol.

Real loop resolves Zellij.

Real loop runs shared required checks.

Missing Git or empty WASM returns a named structured preflight failure.

Real loop then discovers the Git root.

The defensive inline embed guard remains before any byte use and carries the
same released-installer remedy.
