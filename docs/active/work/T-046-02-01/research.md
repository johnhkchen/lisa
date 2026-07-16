# Research — T-046-02-01 runtime resolver and config

## Ticket boundary

T-046-02-01 introduces selection of the Zellij executable used by Lisa.

The selection has three externally named modes: pinned, system, and managed.

The project configuration surface is `.lisa.toml` under `[runtime] zellij`.

An absolute path value selects pinned mode.

The literal `"system"` selects a binary discovered through `PATH`.

The literal `"managed"`, or no setting, selects Lisa's per-version data path.

The resolver must retain mode, detected version, and resolved path for reporting.

The loop must launch the resolved path instead of the command name `zellij`.

Doctor must report the same resolution decision the loop will use.

Managed acquisition, download, checksum verification, and atomic installation
belong to dependent ticket T-046-02-02, not this ticket.

## Existing version contract

`crates/lisa-core/src/version.rs` was added by T-046-01-01.

It owns the `ZellijVersion` domain value.

`ZellijVersion::parse_command_output` accepts the real
`zellij <semantic-version>` output shape.

It rejects missing product names, invalid semantic versions, and extra fields.

`SUPPORTED_ZELLIJ_RANGE` is the single supported-floor declaration.

Its minimum is Zellij 0.43.0.

The range has no maximum.

`classify_zellij_version_output` yields in-range, below-floor, or unparseable.

The parsed variants retain a canonical `ZellijVersion` for diagnostics.

This ticket can depend on that core contract without adding another parser.

## Adjacent floor-enforcement work

T-046-01-02 is active concurrently.

It owns doctor and loop refusal for unsupported system Zellij versions.

Its Research artifact identifies `doctor.rs` and `loop_cmd.rs` as source paths.

Those paths overlap the integration points required by this ticket.

The current committed tree still treats Zellij version output as opaque.

The implementation must therefore re-read these paths before editing and commit.

The runtime resolver should expose a narrow value that the enforcement code can
consume rather than duplicating generic dependency-report machinery.

## Current configuration model

`crates/lisa-cli/src/config.rs` owns `.lisa.toml` parsing and resolution.

`LisaConfig` is a serde-deserialized top-level structure.

Its current sections are `dirs`, `scheduling`, and `agent`.

Each nested section has a default so omitted sections parse successfully.

`load_config` treats a missing `.lisa.toml` as an empty configuration.

An existing unreadable or malformed file returns an actionable string error.

`validate_config` parses the document first as `toml::Value`.

That generic representation is used to find unknown sections and keys.

Unknown values are accumulated as warnings rather than rejected.

The same document is then deserialized into `LisaConfig`.

Semantic constraints, such as nonzero thread counts, are checked afterward.

`ResolvedConfig` contains defaults and all values required by loop execution.

`resolve_config` applies defaults, file values, and limited CLI overrides.

There is no runtime-related CLI flag, so runtime precedence is internal to the
single `[runtime].zellij` value and its default.

## Existing config-template behavior

`default_config_toml` produces the configuration written by `lisa init`.

It contains active directory and scheduling defaults.

Optional agent and scheduling choices are shown as commented examples.

The generated configuration must remain parseable as `LisaConfig`.

`init.rs::upsert_missing_config_keys` preserves existing configuration text.

It currently appends selected missing scheduling keys as comments.

It does not automatically derive new section handling from the default template.

Managed mode is the absence default, so existing configs do not require a
textual migration to remain behaviorally complete.

## Current loop path

`crates/lisa-cli/src/main.rs` loads and validates `.lisa.toml` for `lisa loop`.

It prints every validation warning to stderr.

It calls `resolve_config` and passes the result to `loop_cmd::run_loop`.

`run_loop` first validates `CLAUDE.md`, the ticket directory, and protocol age.

Dry-run returns before external dependency checks and before file writes.

A real run discovers the Git root and all configured agent clients.

It calls doctor's dependency preflight once per possible agent client.

The dependency list currently includes Zellij by its PATH command name.

The loop then writes the embedded plugin, cleans stale cache, and pre-grants
plugin permissions.

It writes `.lisa-layout.kdl` in the project root.

The Unix launch function uses `Command::new("zellij").exec()`.

The non-Unix launch function uses the same bare command name with `.status()`.

Both launch paths therefore delegate executable choice to `PATH` today.

Neither launch function accepts an executable path argument.

## Current doctor path

`crates/lisa-cli/src/doctor.rs` owns dependency checks and human-readable output.

`run_doctor` currently loads config only to select the agent client.

Configuration load failure degrades to the default client.

`build_checks` always includes a generic required Zellij check.

`check_zellij` executes `zellij --version` through `PATH`.

Successful stdout is kept as an opaque first-line string.

Missing or failing execution is reported with an installation hint.

The report has no structured field for executable mode or path.

Doctor also checks project protocol version and cleans Zellij plugin cache.

Cache cleanup uses Zellij's `directories`-crate project identity.

That XDG-aware cache work was completed by T-046-02-03.

## Data-directory conventions

The story requires `$XDG_DATA_HOME/lisa` when `XDG_DATA_HOME` is usable.

The Unix fallback is `$HOME/.local/share/lisa`.

The runtime subdirectory is `runtime/zellij-<version>`.

The executable is naturally the `zellij` file inside that version directory.

The installed runtime must be version-specific so upgrades never overwrite it.

The repository already depends on the `directories` crate in `lisa-cli`.

That dependency is used for Zellij cache semantics, not Lisa data storage.

The ticket states an explicit XDG/fallback contract rather than a platform-
specific application-support path.

`XDG_DATA_HOME` can be absent, empty, relative, or absolute in process state.

An absolute path is the valid XDG form.

Home discovery can fail when `HOME` is missing.

The resolver must surface that as a named error rather than manufacture a path.

## Managed-version boundary

The plugin manifest pins `zellij-tile = "0.43"`.

The workspace lockfile currently resolves the SDK family to 0.43.1.

The story consistently refers to a managed runtime matching that compiled minor.

T-046-02-02 will need a concrete release version for artifact URLs and hashes.

This ticket establishes the path contract that installer work will populate.

Keeping the managed version declaration beside resolver constants gives the
installer one visible value to consume later.

## Executable inspection

Doctor needs a version in all three modes.

Pinned and managed modes cannot obtain that version from a PATH lookup.

The resolved executable itself must therefore be run with `--version`.

System mode needs both PATH discovery and range validation.

`std::process::Command` resolves a bare program name through PATH when spawned.

That does not expose the selected absolute executable path for reporting.

The repository's `which` helper invokes the external `which` command and returns
only a boolean.

The resolver needs a path-returning lookup boundary that is testable with a
controlled PATH.

Canonicalization can turn a discovered or pinned file into an absolute path.

Canonicalization also makes symlink targets explicit in doctor output.

Missing and non-executable files fail when `--version` is invoked even if path
syntax itself is valid.

## Test conventions and constraints

Inline unit tests are common in all three relevant CLI modules.

Temporary directories and executable shell stubs are already used in CLI tests.

Unix permission bits can make test stubs directly executable.

Resolver tests can inject an environment snapshot rather than mutate global PATH
or HOME, avoiding parallel-test races.

Pure path derivation tests can cover XDG present and fallback cases directly.

Stub scripts can emit 0.40.1, 0.43.x, and 0.44.x version output.

The real loop replaces its process, so its launch command should be factored into
a command-construction function for non-destructive assertions.

Doctor report formatting can be tested from a resolved-runtime value without
capturing global stdout.

## Repository state and ownership

The ordinary worktree contains unrelated Lisa bookkeeping and untracked planning
documents.

Those paths are outside this ticket and must remain untouched.

Attempt artifacts belong only under
`.lisa/attempts/T-046-02-01/1/work/`.

Likely ticket-owned source paths are `config.rs`, a new runtime module,
`loop_cmd.rs`, `doctor.rs`, and the binary module declaration in `main.rs`.

The source unit must be committed with exact `lisa commit-ticket` includes.

Ordinary index operations are prohibited by the assignment.
