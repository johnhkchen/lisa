# Design: Git-root-aware completion command

## Decision

Discover the enclosing Git root in the native `lisa loop` process, pass it as
explicit plugin configuration, retain it as a second `State` root, and make the
completion command mapper convert every path from the Lisa project namespace to
the Git-root namespace before emitting argv.

## Options considered

### 1. Keep passing the project root and rely on the CLI

The current CLI already discovers the repository and canonicalizes nested
project-relative completion paths. This is compatible with many executions and
is the smallest code change: no change.

It is rejected because the ticket explicitly assigns command construction to
the plugin contract. The emitted `--path` remains wrong, the argv cannot be
inspected as Git-root-relative, and other consumers of the effect cannot rely on
state meaning what its field names claim.

### 2. Discover the Git root inside WASM by walking `.git`

The plugin could start at `/host` and walk parents until it sees `.git`.

It is rejected because `/host` represents the mounted project namespace, not a
guaranteed mount of its enclosing host directories. A nested parent may be
unobservable. Git worktrees also use `.git` files and repository discovery has
more semantics than searching for a directory name.

### 3. Launch `git rev-parse` asynchronously from the plugin

The plugin has RunCommands permission and could ask the host for the Git root,
then retain it after a `RunCommandResult` event.

This introduces an asynchronous initialization state, correlation protocol,
startup ordering, and behavior for completion requests arriving before the
result. It also creates a second host command solely to rediscover information
the native launcher can provide synchronously. It is rejected as needless
state-machine complexity.

### 4. Discover natively and configure the plugin

The native launcher can run `git -C <project> rev-parse --show-toplevel` before
Zellij starts. It can canonicalize both roots and serialize the Git root in KDL.
The plugin retains the project root from Zellij and the Git root from config.

This respects the WASI boundary, makes both roots explicit, avoids asynchronous
startup, and gives command construction enough information for a pure mapper.
It is selected.

## Discovery behavior

Use a small native helper in `loop_cmd.rs` that invokes Git with `-C` and
`rev-parse --show-toplevel`. Treat command failure, nonzero status, empty output,
or canonicalization failure as a loop startup error.

Completion transactions require Git, so silently falling back to the project
root would defer an inevitable failure and recreate ambiguous state. A named
startup error is preferable.

Dry-run should perform the same discovery before rendering its layout. This
keeps displayed configuration representative and exercises nested roots without
launching Zellij.

## State representation

Add `git_root: PathBuf` to `PluginConfig` with an empty default for tolerant
direct construction and backwards-compatible parsing of old layouts. The
generated layout always sets the field for real loops.

Add `git_root: PathBuf` to plugin `State`, copied from parsed configuration in
`load()`. Keep `project_root` unchanged. The two fields have separate meanings:

- `project_root`: Lisa files, hooks, and host cwd;
- `git_root`: completion transaction root and Git pathspec base.

An empty Git root is a named command-build error. It is not inferred from the
project root because that inference is the original nested-project defect.

## Path mapping

Replace `repository_relative_path` with a completion-specific mapper.

First convert the input to a host-absolute candidate:

- `/host` paths are interpreted beneath `project_root`;
- host absolute paths remain absolute;
- relative paths are interpreted beneath `project_root`.

Then lexically normalize `.` and `..` components. Reject a `..` that would move
above a root. Finally strip the normalized Git root. Reject an empty result and
any candidate outside the Git root.

This produces `games/midsummer/docs/active/...` when:

- project root is `/repo/games/midsummer`;
- Git root is `/repo`;
- sandbox path is `/host/docs/active/...`.

Errors include a stable name, `completion path outside Git root`, plus the path
and Git root. `execute_completion_effect` already promotes this into a visible
ticket-specific activity error and removes pending state.

## Command behavior

`build_completion_command` will:

- require `lisa_bin`, project root, and Git root;
- map ticket and canonical work paths to Git-root-relative values;
- emit `--path <git_root>`;
- continue running the host command with `project_root` as cwd.

The cwd need not equal `--path`; Git receives an explicit transaction root.
Keeping the project cwd also preserves any surrounding command behavior.

## Test design

Core configuration tests cover empty default and map round-trip.

Loop layout tests cover the emitted `git_root` field and a temporary nested Git
repository discovery case.

Plugin unit tests directly construct a nested root and assert the complete argv,
including both Git-prefixed includes. A second case supplies an outside absolute
path and asserts the named rejection.

Existing plugin, CLI, workspace, formatting, and WASM checks guard compatibility.

## Rejected expansions

No changes are needed to `complete_ticket` transaction logic; its independent
canonicalization remains defense in depth. No generic repository abstraction is
introduced because only completion effects consume Git-root-relative pathspecs.
No notification or agent launch paths move to the Git root.
