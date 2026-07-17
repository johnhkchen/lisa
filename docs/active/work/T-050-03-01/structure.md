# Structure — T-050-03-01 client-autodetect

## Change set overview

Modify `crates/lisa-cli/src/detect.rs`.
Modify `crates/lisa-cli/src/config.rs`.
Modify `crates/lisa-cli/src/doctor.rs`.
Modify `crates/lisa-cli/src/loop_cmd.rs`.
Create `crates/lisa-cli/tests/client_autodetect.rs`.
Do not modify `crates/lisa-cli/src/main.rs`.
Do not modify `crates/lisa-core`.
Do not modify `crates/lisa-plugin`.
Do not modify provider launch or hook adapters.

## `detect.rs` responsibility

Keep existing project-type detection unchanged.
Add the CLI environment's agent-executable availability vocabulary.
Define `pub(crate) enum AgentAvailability`.
Derive `Debug`, `Clone`, `Copy`, `PartialEq`, and `Eq`.
Variants are `Neither`, `ClaudeOnly`, `CodexOnly`, and `Both`.
Keep the enum crate-private because it is a CLI resolution detail.

Add `pub(crate) fn detect_agent_availability() -> AgentAvailability`.
Read `std::env::var_os("PATH")`.
Treat missing PATH as no candidate directories.
Call a private executable-presence helper once for `claude`.
Call it once for `codex`.
Pass the two booleans to a pure classifier.
Do not execute any candidate.

Add private `fn classify_agent_availability(claude: bool, codex: bool)`.
Map `(false, false)` to `Neither`.
Map `(true, false)` to `ClaudeOnly`.
Map `(false, true)` to `CodexOnly`.
Map `(true, true)` to `Both`.

Add a private PATH candidate helper.
Iterate with `std::env::split_paths`.
Join each directory with platform-appropriate executable names.
Use filesystem metadata only.
On Unix, require `is_file()` and an execute permission bit.
On Windows, honor PATHEXT candidates.
Keep platform-specific imports behind `cfg` attributes.

Extend the existing `detect.rs` test module.
Add a table test for all four classifier outputs.
Add a Unix test showing a regular non-executable file is not considered present.
Add a Unix test showing an executable fixture is considered present.
Keep these helper tests local and avoid process PATH mutation.

## `config.rs` responsibility

Import `AgentAvailability` and the production detector from `crate::detect`.
Define `pub(crate) enum ClientResolution` near `ResolvedConfig`.
Derive `Debug`, `Clone`, `Copy`, `PartialEq`, and `Eq`.
Variants are `Cli`, `Config`, and `Detected(AgentAvailability)`.
This enum represents transient provenance, not stored project configuration.

Add `pub client_resolution: ClientResolution` to `ResolvedConfig`.
The field can remain public within the binary's module model for struct update syntax.
Set the default to `ClientResolution::Detected(AgentAvailability::Both)`.
That pairs with the existing default Claude value.
Production resolution will always replace the default pair from actual inputs.

Add `impl ResolvedConfig::client_announcement(&self) -> &'static str`.
Match `(self.client, self.client_resolution)` exhaustively.
Return exact static strings for both explicit sources and all detected states.
Use brand capitalization in the leading phrase.
Use exact punctuation, em dash, and reason wording.
Defensively handle structurally inconsistent public field pairs with client-specific explicit strings.

Keep the public `resolve_config` signature unchanged.
Inside it, call `detect_agent_availability`.
Delegate to a testable internal resolution function.
Add `fn resolve_config_with_availability(...) -> ResolvedConfig`.
Give it the existing three arguments plus `AgentAvailability`.
Keep it private to the module; the local test module can access it.

Extract client and provenance together at the top of resolution.
CLI option produces `(client, ClientResolution::Cli)`.
Valid file value produces `(client, ClientResolution::Config)`.
No explicit value maps the injected availability.
Place both values into the `ResolvedConfig` literal.
Leave every unrelated resolution field byte-for-byte unchanged.

Update `default_config_toml` comments only.
Explain automatic PATH selection on omission.
Explain the both-installed Claude default.
Keep the example line commented.

Replace the host-sensitive default-client unit assertion.
Use `resolve_config_with_availability` for all four availability cases.
Pin the selected client and exact announcement for each.
Extend explicit config and CLI precedence tests with conflicting availability.
Pin config-source and CLI-source announcements.
Adjust the template comment assertion if needed.

## `doctor.rs` responsibility

Keep all dependency checks unchanged.
Keep `check_claude` unchanged.
Keep `check_codex` unchanged.
Keep `CheckReport` formatting unchanged.
Keep install-hint strings unchanged.
Keep Codex trust handling unchanged.

In `run_doctor`, retain the one call to `resolve_config`.
Obtain the announcement from that resolved value.
Initialize human-readable output with the announcement and a blank line.
Append the existing dependency report after it.
All later report sections append to the same buffer as today.
Do not print an additional standalone line outside the buffer.
This preserves deterministic output ordering.

Add a small unit test only if formatting is not fully covered by config and integration tests.
The primary doctor contract belongs in the real-binary fixture.

## `loop_cmd.rs` responsibility

Keep configured-provider collection unchanged.
Keep project validation unchanged.
Keep dry-run output unchanged.
Keep dependency preflight and adapter setup unchanged.
Keep generated layout unchanged.

In `run_loop`, print `config.client_announcement()` only on the real path.
Place the print after the `dry_run` early return.
Place it before completion/runtime/dependency preflight side effects.
This makes the resolved choice visible even when startup reports a dependency remedy.
Do not print it again in the later runtime summary.
Do not include it in generated layout or plugin configuration.

Add a focused unit assertion only if integration coverage cannot observe early output.
Existing loop unit helpers use `ResolvedConfig::default` and struct update syntax.
No other loop data structures change.

## `tests/client_autodetect.rs` responsibility

Compile only on Unix because the fixture uses shell stubs and Unix permissions.
Import `env`, `fs`, `PermissionsExt`, `Path`, `Command`, and `Output`.
Define exact announcement constants for all detected cases.
Define the exact existing Claude missing-remedy fragment.

Create a fixture helper that owns a temporary directory.
Create `project`, `bin`, and `home` directories.
Write `CLAUDE.md`.
Create `docs/active/tickets`.
Write a current-version `.lisa.toml`.
Use an absolute Zellij stub path in `[runtime]`.
Set completion to journal.
Optionally write an explicit `[agent] client`.
Create `.codex/hooks.json` so codex-selected loop startup crosses that gate.

Always write executable Git and Zellij stubs.
Git answers `--version` successfully.
Zellij answers `--version` with a supported value and otherwise exits successfully.
Conditionally write Claude and Codex stubs.
Each provider answers `--version` successfully.
Set PATH to only the fixture bin directory.
Set HOME to the fixture home directory.
Run `CARGO_BIN_EXE_lisa` and capture output.

Add a codex-only doctor test.
Assert command success.
Assert exact Codex-only announcement.
Assert Codex appears as an OK dependency.
Assert the Claude install remedy is absent.

Add a claude-only doctor test.
Assert command success.
Assert exact Claude-only announcement.
Assert Claude appears as an OK dependency.

Add a both-installed doctor test.
Assert command success.
Assert exact both-installed announcement.
Assert the selected dependency row is Claude.
Assert no Codex trust section appears.

Add a neither-installed doctor test.
Assert command failure.
Assert exact neither-installed announcement.
Assert the complete existing Claude missing-remedy fragment is present.
This fragment pins the byte-preservation criterion.

Add an explicit-config override test.
Supply only Claude on PATH and configure Codex.
Assert the config-source Codex announcement.
Assert doctor reports the Codex missing remedy rather than selecting Claude.

Add a CLI override test through real loop startup.
Supply only Codex on PATH and pass `--client claude`.
Assert the CLI-source Claude announcement.
Assert preflight reports Claude unavailable.
This proves the flag wins before detection.

Add a loop autodetection announcement test.
Supply only Codex on PATH with no explicit selection.
Run real `loop` rather than dry-run.
Assert the exact Codex-only announcement appears.
Allow the assertion to focus on the announcement if a development build later stops at embedded WASM.
If the build embeds WASM, the Zellij stub exits cleanly and the loop can succeed.

## Commit units

First meaningful unit: detection and resolution.
Exact include paths are `crates/lisa-cli/src/detect.rs` and `crates/lisa-cli/src/config.rs`.
Second meaningful unit: operator surfaces and fixture coverage.
Exact include paths are `crates/lisa-cli/src/doctor.rs`, `crates/lisa-cli/src/loop_cmd.rs`, and `crates/lisa-cli/tests/client_autodetect.rs`.
Phase artifacts remain in the private attempt directory for Lisa publication.
No ordinary Git index command is part of either unit.

## Final diff boundary

The ticket-owned source include list contains exactly five paths.
The absence of provider adapter paths demonstrates N1 scope.
The absence of `.lisa.toml` fixture state outside temporary directories demonstrates no persistence.
The absence of `main.rs` shows CLI parsing precedence remains structurally unchanged.
The absence of `lisa-core` shows the shared provider model remains unchanged.
