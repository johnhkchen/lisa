# Structure — T-046-02-01 runtime resolver and config

## Change summary

Add one focused native runtime module.

Extend the existing config schema with a runtime section and typed request.

Wire doctor and loop to the same resolved runtime value.

Change the launch boundary to accept an absolute executable path.

No source file is deleted.

## File: `crates/lisa-cli/src/runtime.rs`

This new file owns Zellij runtime selection and inspection.

It has no scheduler, ticket, layout, cache, or installation responsibilities.

### Constants

`MANAGED_ZELLIJ_VERSION` is the exact release installed by managed mode.

It is constructed as `ZellijVersion::release(0, 43, 1)`.

The constant's documentation names the 0.43 SDK alignment and installer handoff.

The executable basename remains `zellij` on Unix.

### `ZellijRuntimeRequest`

Crate-visible enum describing resolved configuration intent.

Variants:

- `Managed`;
- `System`;
- `Pinned(PathBuf)`.

Traits: `Debug`, `Clone`, `PartialEq`, and `Eq`.

No process or filesystem state is captured in this type.

### `ZellijRuntimeMode`

Crate-visible enum describing the selected mode in a resolved result.

Variants mirror the request without carrying a path.

It implements `Display` with lowercase stable labels.

It derives copy/equality/debug traits for reporting and tests.

### `ResolvedZellijRuntime`

Crate-visible immutable result structure.

Fields:

- `mode: ZellijRuntimeMode`;
- `version: ZellijVersion`;
- `path: PathBuf`.

All fields are readable by doctor and loop.

The constructor remains internal to the resolver.

### `RuntimeEnvironment`

Private or test-visible structure containing environment inputs.

Fields capture `PATH`, `XDG_DATA_HOME`, and `HOME` as OS-native values.

`from_process` snapshots the current process environment.

Tests construct explicit snapshots, avoiding global environment mutation.

### `managed_zellij_path`

Pure helper receiving environment inputs.

Returns an absolute `PathBuf` or a named string error.

It accepts only an absolute nonempty `XDG_DATA_HOME`.

It otherwise uses an absolute HOME-based `.local/share` fallback.

It appends the version-specific Lisa runtime suffix.

It performs no directory creation.

### `find_system_zellij`

Pure filesystem lookup helper receiving the PATH snapshot.

Iterates `std::env::split_paths` in order.

Finds the platform-appropriate executable candidate.

Canonicalizes the chosen candidate.

Returns a named lookup error if no candidate is found.

### `inspect_zellij`

Process helper receiving an absolute path and mode.

Runs `<path> --version` and captures complete stdout.

Rejects spawn failures and nonzero exits with path-specific errors.

Classifies output through the core version module.

Returns the canonical version only for an in-range result.

Below-floor and unparseable branches produce distinct actionable strings.

### `resolve_zellij_runtime`

Production entry point receiving `&ZellijRuntimeRequest`.

It snapshots process environment and delegates to an injectable inner helper.

Managed mode derives the versioned data path.

System mode performs PATH lookup.

Pinned mode uses its absolute configured path.

All modes normalize the final path and inspect the selected executable.

It returns `ResolvedZellijRuntime` or a named failure.

### Unit tests

Test helpers create executable Zellij stubs in temporary directories.

Tests cover managed XDG and HOME path derivation.

Tests cover missing-home failure.

Tests cover ordered system PATH lookup and absolute normalization.

Tests cover pinned mode ignoring PATH.

Tests cover mode/version/path fields for all modes.

Tests cover 0.40.1 refusal with required-range and remedy text.

Tests cover 0.43.x and 0.44.x acceptance.

Tests cover unparseable and nonzero command output.

## File: `crates/lisa-cli/src/config.rs`

Add `use crate::runtime::ZellijRuntimeRequest`.

### `LisaConfig`

Add a defaulted `pub runtime: RuntimeConfig` field.

### `RuntimeConfig`

New deserialization structure with `pub zellij: Option<String>`.

It derives `Debug`, `Default`, and `Deserialize`.

The raw string preserves the repository's semantic-validation convention.

### `ResolvedConfig`

Add `pub zellij_runtime: ZellijRuntimeRequest`.

Its default value is `Managed`.

`resolve_config` maps the optional raw value to the typed request.

The mapping is infallible after `validate_config`.

### Validation tables

Add `runtime` to `known_top`.

Add a `known_runtime` slice containing `zellij`.

Walk the runtime TOML table and warn for every unknown key.

Validate non-symbolic Zellij values as absolute paths.

### Default config template

Add a `[runtime]` section before `[agent]`.

Document that managed is the default.

Show commented `system` and absolute-path alternatives without activating them.

The template deserializes to a managed request.

### Config tests

Parse and resolve absent, explicit managed, system, and absolute-path values.

Assert the explicit pin has the strongest direct representation.

Assert an unknown runtime key warns but does not fail.

Assert a relative pin fails semantically.

Assert the default template remains parseable and resolves managed.

## File: `crates/lisa-cli/src/main.rs`

Add `mod runtime;` beside the other binary-local modules.

No command-line arguments change.

The Loop command continues to pass only `ResolvedConfig` to `run_loop`.

The Doctor dispatch remains `doctor::run_doctor(&path)`.

## File: `crates/lisa-cli/src/loop_cmd.rs`

### Runtime resolution

After project validation and dry-run exit, call
`runtime::resolve_zellij_runtime(&config.zellij_runtime)`.

Perform this before plugin/layout filesystem writes.

Reuse the returned path through the end of launch.

Print the selected mode, version, and path in startup output.

### Dependency checks

Use the adjacent floor-enforcement API as it exists after T-046-01-02.

Avoid a second generic Zellij PATH check once runtime resolution succeeded.

Agent-client checks remain unchanged.

### Launch functions

Change both `exec_zellij` signatures to receive `zellij_path: &Path`.

Construct the command with `Command::new(zellij_path)`.

Preserve layout argument and current-directory behavior.

Failure messages include the actual path.

### Command construction seam

If needed for testing, add `zellij_command` returning a configured `Command`.

Unix `exec_zellij` calls `.exec()` on it.

Non-Unix `exec_zellij` calls `.status()` on it.

### Loop tests

Assert command programs for managed and pinned absolute paths.

Retain all layout and dry-run tests.

Dry-run tests must not require an installed managed runtime.

## File: `crates/lisa-cli/src/doctor.rs`

This is an integration file shared with active T-046-01-02.

Edits are applied only after re-reading its committed state.

### Config loading

Retain the full `ResolvedConfig` for both client and runtime selection.

Do not silently replace a malformed existing config with default runtime intent.

If current behavior must be preserved for unrelated checks, represent config
failure explicitly in the runtime report.

### Runtime report

Add a formatter for a `ResolvedZellijRuntime`.

The output contains exact labels for mode, version, and path.

The report is included in the dependency section near the Zellij entry.

Resolution errors remain required failures.

### Generic check list

Remove or parameterize the hard-coded PATH-only Zellij check.

The chosen runtime must be inspected only once per doctor invocation.

Agent and optional WASM-target checks retain their existing structure.

### Doctor tests

Build resolved runtime fixtures for all three modes.

Assert each report contains its mode, canonical version, and absolute path.

Assert runtime failure affects the final doctor result.

Preserve cache and Codex-trust tests.

## File: `crates/lisa-cli/src/lib.rs`

No change is expected because runtime selection is binary-internal today.

Integration tests can exercise it through the binary or inline module tests.

If a reusable library boundary becomes necessary, expose only after evidence.

## Artifact: `progress.md`

Track each implementation unit, command result, and concurrency adaptation.

It remains in the attempt-private directory.

## Review artifacts

Write `review.md` and the exact disposition JSON after source commits and tests.

They remain private until Lisa admits and publishes them.

## Source commit boundaries

First meaningful unit: runtime config and resolver.

Expected exact paths:

- `crates/lisa-cli/src/runtime.rs`;
- `crates/lisa-cli/src/config.rs`;
- `crates/lisa-cli/src/main.rs`.

Second meaningful unit: loop and doctor integration.

Expected exact paths:

- `crates/lisa-cli/src/loop_cmd.rs`;
- `crates/lisa-cli/src/doctor.rs`.

If concurrency makes an atomic combined commit safer, all five exact paths may
be included in one isolated transaction, with the reason recorded in progress.

No lockfile or manifest change is expected.

## Dependency direction

`runtime.rs` depends on `lisa-core::version` and the standard library.

`config.rs` depends on the request enum but never invokes the resolver.

`doctor.rs` and `loop_cmd.rs` consume both config intent and runtime results.

The runtime module does not depend on doctor or loop formatting types.

T-046-02-02 can extend runtime managed acquisition without changing config syntax.

## Verification boundary

Run config and runtime focused tests first.

Run doctor and loop focused tests after integration.

Run `cargo test -p lisa-cli` and `just check` before Review.

Inspect exact diffs and the ordinary index before every isolated commit.

Confirm every ticket-owned source path is clean after the final source commit.
