# Structure: Git-root-aware completion command

## Modified files

### `crates/lisa-core/src/types.rs`

Extend `PluginConfig` with `git_root: PathBuf`.

`PluginConfig::new` initializes it empty so older/manual configuration remains
parseable. `PluginConfig::from_config_map` accepts the `git_root` layout key.
Add a focused round-trip test.

This field is transport configuration. It does not alter DAG or ticket types.

### `crates/lisa-cli/src/loop_cmd.rs`

Add native Git-root discovery near loop startup.

Change layout generation to receive the discovered root and emit:

`git_root "<absolute-root>"`

Both real and dry-run flows discover before generating. Existing layout tests
pass a root explicitly. Add coverage for the emitted key and nested discovery.

No new dependency is required; use `std::process::Command`.

### `crates/lisa-plugin/src/lib.rs`

Add a `git_root` field beside `project_root` in `State`.

During `load`, retain `config.git_root` separately while continuing to capture
the Lisa project root from Zellij's initial cwd.

Add a lexical absolute-path normalization helper local to the module. Replace
the project-relative mapper with a completion Git-relative mapper that handles
sandbox absolute, host absolute, and relative inputs.

Update `build_completion_command` to use `git_root` as `--path` and mapped
Git-relative ticket/work arguments.

Add nested-root argv and outside-root rejection unit tests near completion tests.

## Interfaces

`PluginConfig.git_root: PathBuf` is public like the existing directory fields.

`discover_git_root(project_root: &Path) -> Result<PathBuf, String>` remains
private to `loop_cmd`.

`normalize_absolute_path(path: &Path) -> Result<PathBuf, String>` remains
private to the plugin module.

`State::completion_repository_relative_path(&self, path: &Path)
 -> Result<PathBuf, String>` remains private.

No CLI arguments, persisted ticket schema, or core completion event types change.

## Data flow

1. `lisa loop` receives the Lisa project root.
2. Native Git discovery resolves the enclosing repository root.
3. Layout KDL transports the absolute Git root.
4. Plugin load retains both root values.
5. Completion effect supplies sandbox ticket/work paths.
6. The mapper projects sandbox paths through the project root.
7. Normalized host paths are stripped against the Git root.
8. The command passes Git root plus Git-root-relative includes.
9. Existing CLI validation canonicalizes and checks the same boundary again.

## Failure boundaries

Loop startup fails visibly when no repository can be discovered.

Command construction fails when either root is absent, the input cannot be
normalized, the normalized input equals the repository root, or it lies outside
the Git root.

The completion executor continues to own pending rollback and visible activity
logging for command-build failures.

## Change ordering

1. Add the shared configuration carrier.
2. Add native discovery and layout transport.
3. Retain and consume the root in plugin command construction.
4. Add focused tests at each boundary.
5. Format and run targeted tests.
6. Commit all three coupled paths in one isolated Lisa transaction.
7. Run broader verification and write Review artifacts.

The three source files form one meaningful unit: omitting any one leaves either
the layout, parser, or consumer contract incomplete.

## Non-owned files

Do not modify the active ticket frontmatter, provenance ledger, shared admitted
work directory, or pre-existing `crates/lisa-plugin/docs/` tree.
