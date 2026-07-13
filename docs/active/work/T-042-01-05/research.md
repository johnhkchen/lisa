# Research: Git-root-aware completion command

## Ticket boundary

T-042-01-05 fixes completion effects when a Lisa project is nested inside a
larger Git repository. The reported production shape is an Arcade repository
whose Lisa project lives at `games/midsummer`.

The ticket has one acceptance criterion: completion commands must distinguish
the Lisa project root from the Git repository root, pass the Git root to
`complete-ticket --path`, convert the ticket and work paths to normalized
Git-root-relative values, and visibly reject paths outside the Git root.

The ticket starts in Research. Its prerequisite T-042-01-01 is complete at
`dedd6a1`; that work centralized typed completion launches in the plugin.

## Completion flow

`crates/lisa-plugin/src/lib.rs` owns scheduler state and Zellij host effects.
`State::dispatch_completion` reduces typed completion requests. The only host
launch boundary is `State::execute_completion_effect`.

The executor gets the ticket path from the in-memory DAG, records pending
authority, calls `build_completion_command`, and invokes Zellij's
`run_command_with_env_variables_and_cwd`.

`build_completion_command` emits:

- the configured Lisa binary;
- `complete-ticket`;
- `--path`;
- ticket identity and completion message;
- `--ticket-file`;
- `--work-dir`.

The command currently uses `project_root` for `--path`. Both include paths are
produced by `repository_relative_path`.

## Current root model

`State` has one root field, `project_root`. `load()` sets it from
`get_plugin_ids().initial_cwd`. The CLI starts Zellij with `current_dir(root)`,
so this value represents the Lisa project root.

The field also correctly supplies the cwd for notification hooks and agent-side
host commands. Those uses are project-relative and are not Git pathspec uses.

The WASI plugin sees the Lisa project through `/host`. Relative configured
ticket, story, and work directories are rewritten under `/host` during load.
Consequently a scanned ticket normally has a path such as
`/host/docs/active/tickets/T-042-01-05.md` even when its host path is
`/repo/games/midsummer/docs/active/tickets/T-042-01-05.md`.

`repository_relative_path` currently strips `/host` first. That always yields
`docs/active/...`, losing the `games/midsummer` prefix required by Git.

## CLI launch boundary

`crates/lisa-cli/src/loop_cmd.rs` validates the Lisa root, prepares the embedded
WASM, generates `.lisa-layout.kdl`, and execs Zellij from the Lisa root.

The generated plugin configuration carries relative ticket/story/work paths,
timeouts, client routing, provider caps, and the absolute Lisa binary path. It
does not carry a Git repository root.

This native pre-exec boundary has normal host filesystem and process access.
The WASI plugin does not have reliable access to directories above its `/host`
project mount. Therefore enclosing-repository discovery is naturally available
before layout generation, but not by walking upward from `/host` in the plugin.

## Shared configuration

`crates/lisa-core/src/types.rs` defines `PluginConfig`, including defaults and
lenient parsing from the Zellij configuration map. Root information beyond the
ticket/story/work paths is absent.

Adding a configuration field affects the constructor, parser, equality, and
tests that use struct update syntax. Existing test fixtures mostly use
`..PluginConfig::new()`, limiting migration cost.

The layout is textual KDL. Absolute paths inserted into quoted KDL values need
the same platform assumptions already made for `lisa_bin` and the WASM path.

## Native completion transaction

`crates/lisa-cli/src/commit_transaction.rs` independently discovers the Git
root from the requested `--path`. It already contains
`completion_repo_relative_path`, added in commit `bab60da`.

That helper treats supplied ticket/work values as relative to the requested
root, canonicalizes them, and then expresses them relative to the discovered
Git root. This defense makes the CLI tolerant of a nested project root, but it
does not make the plugin's emitted command satisfy the new explicit contract.

The command requested by this ticket should pass the Git root and already
Git-root-relative include paths. The CLI remains a second validation boundary.

## Path constraints

Completion paths may arrive in plugin state in two common forms:

- sandbox absolute paths below `/host`;
- host absolute paths below the Lisa project root in native tests.

Relative paths also occur in directly constructed fixtures. A correct mapper
must interpret sandbox and relative values against the Lisa project root, then
normalize components and strip the Git root.

Merely stripping `/host` is insufficient for nested projects. Merely stripping
the project root also yields project-relative rather than repository-relative
paths. Prefix checks must happen after normalization so `..` cannot escape an
otherwise acceptable textual prefix.

`std::path::Component` is available on both native and WASM targets and supports
lexical normalization without filesystem access. Prefix/root components need
platform-aware handling through `PathBuf` rather than string concatenation.

An outside-root failure travels from `build_completion_command` to
`execute_completion_effect`. In production the executor removes the pending
entry and logs `Cannot start completion for {ticket}: {error}` as an
`ActivityEvent::Error`, making a named mapping error visible on the dashboard.

## Tests and workspace state

The plugin has native unit tests in the large `lib.rs` test module. The existing
Review completion test records typed effects but intentionally tolerates command
construction failure in native tests, so it does not inspect argv.

`loop_cmd.rs` tests generated KDL configuration. `types.rs` tests configuration
defaults and map parsing. These are the direct regression surfaces.

The ordinary index is empty. Lisa-managed ticket/provenance files are modified,
and `crates/lisa-plugin/docs/` is a pre-existing untracked path. They are not
owned by this ticket and must be preserved.

Ticket source commits must use `lisa commit-ticket` with exact repository paths.
Attempt artifacts remain private for Lisa to admit and are not source commits.

## Boundaries and assumptions

- The root passed to `run_loop` is the Lisa project root.
- Completion ticket and work entries exist when a completion is launched.
- The loop must run inside a Git repository for isolated completion to work.
- Notification hooks must continue using the Lisa project root.
- CLI-side canonicalization remains valuable defense in depth.
- No ticket phase or status frontmatter is agent-owned.
