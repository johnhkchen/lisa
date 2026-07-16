# Research — T-046-04-02

## Ticket boundary

T-046-04-02 concerns agent-facing remediation strings and two missing CLI
preflight checks.

The ticket begins in Research and requires all six RDSPI phases in one pass.

Phase artifacts belong in this attempt-private work directory.

Lisa, rather than the agent, owns ticket phase and status transitions.

Ticket-owned source changes must be committed with `lisa commit-ticket` and
exact repository-relative include paths.

## User installation contract

`README.md` is the current source for installing a released Lisa binary.

Its first installation method is the cargo-dist shell installer:

`curl --proto '=https' --tlsv1.2 -LsSf https://github.com/johnhkchen/lisa/releases/latest/download/lisa-cli-installer.sh | sh`

The README explicitly says Rust is not required to use Lisa.

It also tells agents not to build from source when the goal is installation or
use.

Homebrew is a secondary macOS installation path.

Repository build instructions are scoped to developing Lisa.

These statements were established by the preceding documentation ticket,
T-046-04-01.

## Doctor command entry point

`crates/lisa-cli/src/main.rs` defines the `doctor` subcommand.

The command accepts a project path and dispatches to
`doctor::run_doctor(&path)`.

`crates/lisa-cli/src/doctor.rs` contains dependency discovery, report
formatting, project-version reporting, cache cleanup, and Codex trust seeding.

`run_doctor` first loads and resolves `.lisa.toml`.

The selected agent client comes from that resolved configuration.

The configured Zellij runtime is checked separately from the general
dependency vector so its mode, version, and resolved executable path can be
reported.

`build_checks(client)` currently returns the selected agent check plus an
optional Rust WASM-target check.

The selected agent is required.

The Rust target is optional and skipped when `rustup` is absent.

There is no `git` entry in `build_checks`.

There is no embedded-plugin entry in `build_checks`.

## Dependency model

Each dependency is represented by `DependencyCheck`.

It carries a static display name, a required flag, and a closure returning a
`CheckResult`.

`CheckResult::Found` carries version/detail text.

`CheckResult::NotFound` carries an install hint.

`CheckResult::Unsupported` carries a description and remedy.

`CheckResult::Skipped` carries a reason.

`CheckReport` combines the dependency metadata with its evaluated result.

Its `Display` implementation is the common formatting boundary for doctor and
loop preflight failures.

Missing required tools render as `not found` and an `Install:` line.

Unsupported required components render as `unsupported`, a description, and a
`Remedy:` line.

`has_failures` treats required missing or unsupported results as failures.

Optional missing or skipped results do not fail doctor.

`check_required_deps` evaluates the same `build_checks` vector for real loop
preflight and returns formatted required failures.

## Command discovery

`get_command_version` invokes a command with supplied arguments.

It returns the first stdout line only when spawning succeeds and the command
exits successfully.

It returns `None` for a missing executable, a nonzero exit, or another spawn
failure.

`which(name)` separately invokes the external `which` program.

The selected agent checks use `get_command_version` directly.

The optional Rust-target check uses `which("rustup")` before invoking rustup.

There is no current call to `git --version` in doctor.

## Zellij behavior

Zellij handling changed after the ticket text was drafted.

`check_zellij_runtime` delegates to `runtime::resolve_zellij_runtime`.

A successful result reports runtime mode, version, supported range, and path.

A failure is `Unsupported` and tells the user to select Lisa's managed runtime
or configure a compatible absolute path.

The default managed mode is therefore already the main user-facing Zellij
remedy.

Test-only version-classification helpers retain a static-binary remedy for
below-floor and unparseable versions.

The literal phrase prohibited by acceptance remains in generic unit-test
fixtures in `doctor.rs`.

Those fixtures exercise report formatting rather than the production Zellij
resolver.

## Embedded WASM production path

`crates/lisa-cli/src/templates.rs` defines `PLUGIN_WASM` with `include_bytes!`
from the CLI build output directory.

`crates/lisa-cli/build.rs` populates that output file.

The build script looks for the release plugin at
`target/wasm32-wasip1/release/lisa.wasm`.

When that file exists, the build script verifies the WebAssembly header and
copies the bytes.

When the file is missing in an ordinary developer CLI-only build, the build
script writes an empty placeholder.

Release CI can set `LISA_REQUIRE_EMBEDDED_WASM=1` to make a missing plugin a
build failure.

The placeholder intentionally permits `cargo build -p lisa-cli` for developer
work.

The resulting CLI remains executable, so runtime checks must distinguish it
from a complete distribution.

## Current loop handling

`crates/lisa-cli/src/loop_cmd.rs` imports `PLUGIN_WASM`.

For a real loop, it validates project files and protocol version.

It then calls `discover_git_root` before resolving Zellij or checking general
dependencies.

`discover_git_root` invokes `git rev-parse --show-toplevel`.

If `git` is absent, that call exposes a command-spawn error before dependency
preflight can render a structured report.

After Git discovery, the loop resolves Zellij and calls
`doctor::check_required_deps` for every configured agent provider.

It later checks `PLUGIN_WASM.is_empty()` directly.

The current empty-WASM error says the plugin is not embedded.

It describes `cargo install` as incomplete and recommends cloning the
repository and running `just release`.

That source-build instruction conflicts with the released-install contract.

The loop check occurs before writing the plugin, layout, cache entries, or
launching Zellij.

Dry-run intentionally bypasses external dependency and embedded-WASM checks.

## Tests and test boundaries

Most doctor tests are unit tests inside `doctor.rs`.

Helpers create found, missing, unsupported, and skipped dependency closures.

Existing tests cover report formatting, failure classification, selected-agent
composition, Zellij version details, cache cleanup, and Codex trust behavior.

Several generic missing-dependency fixtures use the prohibited Zellij Cargo
command as arbitrary sample text.

`crates/lisa-cli/src/loop_cmd.rs` also has an internal unit-test module.

It covers dependency-preflight formatting, layout generation, Git-root
discovery, hashing, and dry-run validation.

There is no isolated formatter or unit test for the empty-WASM loop message.

`crates/lisa-cli/tests/zellij_version_preflight.rs` runs the compiled CLI with
stub executables on Unix.

Its helper creates a project and prepends a temporary binary directory to the
existing PATH.

The integration tests currently accept `WASM plugin not embedded` as the
expected terminal failure after supported Zellij preflight.

A binary built for tests can therefore contain the empty placeholder.

That test binary is suitable for exercising doctor detection of the placeholder
without constructing alternate bytes.

## Repository state and ownership constraints

The worktree contains unrelated modified and untracked Lisa metadata and
planning documents.

Those paths predate this ticket and are not ticket-owned source changes.

The relevant CLI source paths are clean before implementation.

The active ticket file is itself untracked in the shared worktree and is owned
by Lisa's orchestration flow.

Research found additional historical and survey documents that quote the old
Zellij Cargo command.

Those occurrences describe prior behavior and are outside the runtime string
production boundary.

The source-code occurrences are all generic tests in `doctor.rs`.

## Constraints surfaced

Doctor and loop should share dependency classification where practical because
the loop already consumes doctor-formatted failures.

Git discovery currently precedes that shared preflight, so adding a check alone
does not change the loop's earliest failure.

The embedded byte slice has static lifetime and can be evaluated without I/O.

The empty placeholder is a binary packaging defect, not a missing developer
tool or optional target.

The shell-installer remediation must be asserted at the string-production
boundary so future edits cannot silently restore a source-build spiral.

Tests must avoid requiring a network connection, Zellij launch, or a plugin
build.

No ticket phase/status edits are permitted.

No ordinary Git index or ordinary commit operation is permitted for the source
changes.
