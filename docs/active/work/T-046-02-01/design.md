# Design — T-046-02-01 runtime resolver and config

## Objective

Introduce one runtime-selection contract shared by loop and doctor.

It must preserve the configured intent, resolve an absolute executable path,
inspect the selected executable, and retain the mode/version/path decision.

It must not download or install a runtime; that is T-046-02-02's boundary.

## Option 1 — keep runtime selection inside `loop_cmd.rs`

The loop could match the config string immediately before `exec_zellij`.

Advantages:

- minimal new module structure;
- direct access to the launch site;
- few public or crate-visible types.

Disadvantages:

- doctor would need to duplicate path and version resolution;
- managed data-path rules would be split across commands;
- the installer ticket would lack a stable resolver boundary;
- testing launch behavior would still require reaching deep loop setup;
- diagnostic wording could drift between loop and doctor.

Rejected because identical runtime identity must appear on both surfaces.

## Option 2 — put runtime selection in `config.rs`

`resolve_config` could turn `[runtime].zellij` directly into a filesystem path.

Advantages:

- the final configuration would contain everything the loop needs;
- configuration precedence would be implemented in one function;
- no second resolution call would be needed.

Disadvantages:

- config parsing would acquire filesystem, environment, PATH, and process IO;
- status and validate consumers would unexpectedly execute Zellij;
- pure configuration unit tests would need executable fixtures;
- config resolution does not currently receive an environment abstraction;
- doctor could no longer distinguish parse intent from runtime availability.

Rejected because `.lisa.toml` resolution should remain deterministic and pure.

## Option 3 — dedicated native runtime module

Add `crates/lisa-cli/src/runtime.rs` for runtime intent, path derivation, command
inspection, support classification, and the resolved result type.

`config.rs` stores a typed runtime request inside `ResolvedConfig`.

Loop and doctor call the runtime resolver at their command boundaries.

Advantages:

- one implementation defines mode, version, and path;
- configuration remains free of IO;
- loop and doctor consume the same value;
- T-046-02-02 can extend managed acquisition at one boundary;
- focused unit tests do not need the full CLI or scheduler;
- system rejection can reuse the core supported-range contract;
- process/environment seams can be injected without global mutation.

Disadvantages:

- adds a module and several small types;
- `main.rs` must declare the module for the binary crate;
- adjacent doctor work must integrate with the new error shape.

Chosen because it keeps policy cohesive and command consumers thin.

## Configuration representation options

### Raw string throughout

`ResolvedConfig` could retain `String`, defaulting to `"managed"`.

This is simple but forces every consumer to interpret special literals and
absolute paths repeatedly.

It also permits an invalid relative pin to survive until late execution.

Rejected for the resolved layer.

### Untagged serde enum

Serde could deserialize `"system"`, `"managed"`, or a path directly into an
enum with custom deserialization.

This provides an early type but makes parse errors less consistent with the
repository's validation approach.

It also couples TOML syntax to the runtime domain type.

Rejected because the existing config convention keeps strings raw until
semantic validation and resolution.

### Raw optional config plus typed resolved intent

`RuntimeConfig` contains `pub zellij: Option<String>`.

`ResolvedConfig` contains `pub zellij_runtime: ZellijRuntimeRequest`.

`ZellijRuntimeRequest` has `Managed`, `System`, and `Pinned(PathBuf)` variants.

`resolve_config` maps absence to `Managed`, literals to their variants, and any
other validated absolute string to `Pinned`.

Chosen because it mirrors the established agent-client pattern while removing
string interpretation from downstream consumers.

## Config validation

Add `runtime` to the known top-level section list.

Add `zellij` as the only known `[runtime]` key.

Unknown runtime keys produce warnings in the existing warning collection.

They never prevent typed deserialization because serde ignores unknown fields.

`"managed"` and `"system"` are valid symbolic values.

Every other value denotes a pinned path.

Pinned paths must be absolute.

A relative value is rejected with a message naming `[runtime].zellij` and the
accepted `managed`, `system`, or absolute-path forms.

This is semantic value validation, not unknown-key rejection.

## Default and precedence

No `[runtime]` section means `Managed`.

`[runtime] zellij = "managed"` explicitly selects the same mode.

`[runtime] zellij = "system"` overrides the default with PATH lookup.

Any absolute path overrides both symbolic choices as a pin.

There is only one TOML scalar, so there are no simultaneously populated fields
whose ordering could be ambiguous.

Tests will still assert all four input states to protect the stated precedence.

The default template documents the three forms with managed active as an
explanatory default and system/path examples commented.

Existing configuration files need no migration because absence already selects
managed mode.

## Resolved runtime value

Define `ResolvedZellijRuntime` with:

- `mode: ZellijRuntimeMode`;
- `version: ZellijVersion`;
- `path: PathBuf`.

`ZellijRuntimeMode` contains `Managed`, `System`, and `Pinned`.

Its display strings are exactly `managed`, `system`, and `pinned`.

The resolved path is always absolute.

The version is parsed from the selected executable's own `--version` output.

This prevents a pinned filename or managed-directory name from masquerading as
the runtime actually executed.

## Managed runtime path

Declare one exact managed release version in the runtime module.

Use 0.43.1, the patch version resolved for the compiled 0.43 SDK family.

The directory name is `zellij-0.43.1`.

The full suffix is `lisa/runtime/zellij-0.43.1/zellij`.

If `XDG_DATA_HOME` is present, nonempty, and absolute, use it as the data root.

Otherwise use `$HOME/.local/share`.

If neither provides a usable base, return a named managed-path error.

Do not create directories in this ticket.

Do not fall back from a missing managed executable to system mode.

That fail-closed behavior protects the story's no-unvetted-fallback contract.

T-046-02-02 will populate or acquire this exact path before inspection.

## System lookup

System mode searches the supplied PATH directories in order.

It checks the normal executable filename for the target platform.

The first executable candidate is canonicalized to an absolute path.

Lookup failure names system mode and `zellij` on PATH.

The resolver invokes the resulting path, not the bare command, for inspection
and eventual execution.

This freezes the PATH decision so doctor reporting and loop launch agree even
if the working directory later changes.

## Pinned lookup

Pinned mode begins with a validated absolute path.

It canonicalizes the path when possible.

If canonicalization or execution fails, the error names the configured path.

It never consults PATH and never falls back to managed mode.

This gives the explicit path the strongest precedence and deterministic failure.

## Version inspection and enforcement

The selected executable is run once with `--version`.

Nonzero exit, spawn failure, and invalid UTF-8-lossy output become named errors.

Classification delegates to `lisa_core::version`.

In-range output returns the canonical version.

Below-floor output returns an error naming detected version, required range,
runtime mode, and the managed-runtime remedy.

Unparseable output returns a distinct error naming the raw output and required
range.

Although the ticket explicitly calls out system rejection, the same safety
policy applies to pinned and managed binaries because the plugin protocol does
not become compatible merely because a path was explicit.

## Doctor integration

Doctor loads the whole resolved config rather than extracting only the client.

It resolves one Zellij runtime and formats a dedicated runtime report.

The report contains `mode`, `version`, and `path` labels on one readable block.

The generic dependency list no longer performs a second PATH-only Zellij check.

Resolution failure is represented as a required dependency failure so doctor
prints its other checks and returns nonzero.

The report helper is pure and unit-testable from a runtime result.

## Loop integration

Real loop mode resolves Zellij before runtime side effects.

Dry-run remains free of external dependency requirements.

Agent dependency checking remains in doctor's existing generic machinery.

The loop passes `resolved_runtime.path` to `exec_zellij`.

Both Unix and non-Unix launch functions accept `&Path` for the executable.

A pure command builder exposes program and arguments for tests without exec.

Managed and pinned launch tests assert the exact absolute program path.

## Concurrency strategy

T-046-01-02 is allowed to finish its doctor/loop enforcement changes first if
it reaches implementation during this ticket.

Before editing overlapping paths, re-read the committed versions.

Runtime-specific policy remains in the new module.

Shared files should receive only wiring changes.

If the adjacent ticket adds an unsupported-result type, adapt the runtime error
into it instead of restoring older generic structures.

## Rejected scope

No network client, archive parser, checksum, retry policy, or atomic rename.

No apt companion-runtime preference; that belongs to T-046-05-01.

No cache-path changes; T-046-02-03 already owns them.

No new runtime CLI flag.

No ticket phase or status edits.
