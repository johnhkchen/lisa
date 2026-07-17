# Research — T-050-03-01 client-autodetect

## Ticket boundary

The ticket changes how an unset loop client is selected.
The requested inputs are the two executable names `claude` and `codex` on `PATH`.
The requested output remains the existing `AgentClient` provider value.
The ticket explicitly excludes version probes, account state, network access, and adapter changes.
It also excludes persistent detection state.
Explicit `.lisa.toml` configuration and the loop `--client` flag retain precedence.
The visible surfaces are `lisa doctor` and real loop startup.
The default when both executables exist remains Claude.
The fallback when neither exists remains Claude so doctor can retain its existing Claude install remedy.

## Shared client vocabulary

`crates/lisa-core/src/client.rs` defines `AgentClient`.
Its variants are `Claude` and `Codex`.
It derives `Copy`, `Eq`, `Hash`, serialization traits, and `Default`.
The default variant is Claude.
`AgentClient::parse` accepts case-insensitive `claude` and `codex` strings.
`AgentClient::VALID` supplies the accepted names for configuration validation.
`AgentClient::as_str` and `Display` emit lowercase names.
`AgentClient::context_file` preserves the provider-specific project context boundary.
Nothing in the shared type records how a client was chosen.
Provider adapters consume the selected enum and do not need selection provenance.

## Configuration parsing and resolution

`crates/lisa-cli/src/config.rs` owns `.lisa.toml` parsing and CLI-side resolution.
`LisaConfig.agent.client` is an `Option<String>`.
The raw string allows semantic validation to produce the established actionable error.
`validate_config` parses an explicit client with `AgentClient::parse`.
Unknown `[agent]` keys remain warnings.
`ResolvedConfig.client` is a concrete `AgentClient`.
`ResolvedConfig::default` obtains its client from `AgentClient::default`.
`resolve_config` accepts the parsed config, a max-thread override, and an optional CLI client.
Current client precedence is `--client`, then `[agent].client`, then `ResolvedConfig::default().client`.
The final `unwrap_or(defaults.client)` is the unconditional Claude behavior named by the ticket.
All other resolved settings are independent of the client-selection expression.
`default_config_toml` comments currently describe Claude as the default and Codex as opt-in.
The generated `[agent]` example is commented, so freshly initialized projects are unconfigured.

## Configuration tests

The unit tests in `config.rs` exercise defaults, explicit config, and CLI precedence.
`test_resolve_client_default_is_claude` pins the current unconditional fallback.
`test_resolve_client_from_config` pins explicit file configuration.
`test_resolve_cli_client_overrides_config` pins CLI-over-file precedence.
`test_default_config_toml_agent_example_is_inert` confirms initialization does not opt in.
Many unrelated config tests call `resolve_config` and inspect other fields.
Tests run in parallel and therefore should not mutate process-wide `PATH` inside unit tests.
An injectable availability value would allow deterministic resolution tests without environment mutation.

## Project detection module

`crates/lisa-cli/src/detect.rs` owns environment-derived project detection.
It currently recognizes Rust, Node, Go, Python, and unknown projects from marker files.
Detection functions are side-effect-free reads of the supplied project root.
The module contains focused unit tests built from temporary filesystem fixtures.
The file has no dependency on client adapters or configuration parsing.
It is already imported as `mod detect` by the CLI binary.
The ticket explicitly points to this file as the pattern and location for PATH detection.
No current function in this module inspects `PATH`.

## Existing executable discovery

`crates/lisa-cli/src/doctor.rs` contains a `which(name)` helper.
That helper executes the external `which` program and suppresses its output.
Doctor uses it only to decide whether the optional Rust WASM target check should run.
Provider dependency checks do not use `which`.
`check_claude` executes `claude --version`.
`check_codex` executes `codex --version`.
Those functions distinguish a usable provider command from a missing one for doctor reporting.
The new selection requirement is narrower: PATH presence only, with no version execution.
A controlled PATH containing only fixture binaries may not contain an external `which` executable.
Direct PATH-directory inspection can therefore be tested without relying on host utilities.

## Doctor flow

`crates/lisa-cli/src/doctor.rs::run_doctor` loads and resolves configuration itself.
It calls `resolve_config` with no CLI client because doctor has no `--client` flag.
It copies `resolved_config.client` into a local value.
`build_checks(client)` selects exactly one provider dependency check.
Claude failures render the existing Anthropic install URL.
Codex failures render the existing npm command and Codex documentation URL.
Doctor also checks Git, the resolved Zellij runtime, embedded WASM, and optional Rust state.
Doctor formats dependency reports through `CheckReport::fmt` and `format_report`.
The provider missing-remedy bytes are produced by the existing provider check and formatter.
Codex selection additionally enables the best-effort directory-trust section.
Doctor writes its assembled report once with `println!`.
The report currently has no sentence naming why the client was selected.

## Loop command construction

`crates/lisa-cli/src/main.rs` parses `lisa loop --client` as an optional string.
The string is validated with `AgentClient::parse` before config resolution.
The resulting optional enum is passed to `resolve_config`.
This is the only CLI override path for loop client selection.
`lisa doctor` goes directly to `doctor::run_doctor` and has no equivalent override.
The main command does not otherwise inspect provider availability.

## Loop startup flow

`crates/lisa-cli/src/loop_cmd.rs::run_loop` receives a `ResolvedConfig`.
It validates project files and the project protocol before external startup work.
Dry-run exits into `run_dry` before external dependency checks and process launch.
Real startup resolves completion and Zellij runtime decisions.
`configured_clients` starts with the resolved default client.
It adds provider routes explicitly present on tickets.
Dependency preflight checks every provider the board can route to.
Codex in that set activates the Codex hooks-file requirement and trust pre-seeding.
The resolved default client is passed into generated plugin configuration.
Provider-specific layout and scheduler behavior already branch on the enum.
The startup report begins with `Lisa loop starting...` after layout generation.
It currently prints runtime, path, thread, and provider-cap details but not client choice.
The loop has no access to the original raw config after resolution.
Selection provenance must therefore travel transiently with resolved configuration if startup explains why.

## Routing boundary

Per-ticket `agent:` routing is resolved independently by `lisa_core::route::resolve_route`.
The resolved loop client supplies the default only when a ticket does not name a route.
Mixed-provider preflight intentionally checks both installed commands when tickets require both.
Autodetection does not remove or merge that behavior.
Provider caps remain keyed by provider names and are unaffected.
The generated layout still receives one concrete default client.
This preserves the ticket's N1 constraint: selection chooses a provider without changing its adapter.

## Existing integration-fixture patterns

`crates/lisa-cli/tests/zellij_version_preflight.rs` runs the built `lisa` binary.
It creates temporary projects and executable shell stubs.
It constructs PATH from a fixture bin directory and optionally the host PATH.
It pins Zellij output without invoking a real Zellij session.
It asserts doctor and loop stdout/stderr behavior.
`crates/lisa-cli/tests/seal_visibility.rs` uses a similar temporary-project fixture.
It writes `.lisa.toml`, `CLAUDE.md`, tickets, and provider/runtime stubs.
Unix executable permissions are set with `PermissionsExt`.
These patterns support a dedicated client-autodetection fixture with complete PATH control.
A fixture-only PATH can contain provider, Git, and Zellij stubs without leaking host agents.

## Test and build boundaries

The CLI crate embeds WASM through its build script.
Developer builds can contain an empty placeholder if release WASM is absent.
Doctor treats an empty embedded plugin as a required dependency failure.
Existing integration tests already account for the repository's test/build setup around that check.
Loop tests can observe an early startup announcement even when later startup stops at a placeholder.
The workspace guidance names `cargo test --workspace` as the complete native suite.
`just check` adds the WASM target check and is the repository's quick combined verification.

## Worktree and ownership constraints

The ordinary worktree already contains modified Lisa journals and ticket files.
Those paths are scheduler-owned and are not part of this ticket's source include list.
The ticket assignment requires phase artifacts under the private attempt work directory.
Lisa publishes admitted artifacts later.
Ticket-owned source commits must use `lisa commit-ticket` with exact relative paths.
Provider adapter files are outside the expected diff.
No ticket phase or status frontmatter is agent-owned.

## Observed invariants

An explicit CLI value is already parsed before resolution.
An explicit file value is already validated before resolution.
Neither override requires provider presence to be selected.
Doctor is responsible for reporting when the explicitly selected binary is absent.
Loop preflight is responsible for rejecting unavailable providers before scheduling.
The neither-installed fallback must remain Claude to retain the existing doctor remedy.
The both-installed outcome must remain Claude to preserve the stated default.
Only the codex-only environment changes the concrete fallback relative to current behavior.
Reason text is presentation state, not project state.
Detection belongs before provider dependency probing and is reusable by doctor and loop resolution.
