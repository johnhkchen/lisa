# Review: Git-root-aware completion command

## Disposition

Pass.

T-042-01-05 now distinguishes the Lisa project root from its enclosing Git
repository root throughout completion effect construction. A project at
`games/midsummer` emits repository includes beginning with
`games/midsummer/docs/active/...`, passes the Git root to `--path`, and rejects
outside-root paths with a named error.

The focused regressions, full workspace suite, WASM Clippy, formatting, diff,
commit ownership, and index-hygiene checks pass. No blocking issue remains.

## Source change

Modified:

- `crates/lisa-core/src/types.rs`;
- `crates/lisa-cli/src/loop_cmd.rs`;
- `crates/lisa-plugin/src/lib.rs`.

No production file was created or deleted. No manifest or lockfile changed.

The implementation commit is:

`f48134cdb7112eb66181120c73d2917e7cd31da7`

It was created through the repository-built `lisa commit-ticket` transaction
and contains exactly the three paths above. The installed Lisa binary was older
and did not expose `commit-ticket`; it made no Git state change.

## Architecture delivered

### Root discovery and transport

The native loop launcher now discovers the enclosing repository with:

`git -C <project-root> rev-parse --show-toplevel`

It checks Git's status, rejects empty output, canonicalizes the result, and
fails real loop startup with a named error if repository discovery is not
possible.

Layout generation receives the discovered absolute path and writes it as the
plugin's `git_root` configuration value.

Dry-run retains its inspection role for freshly initialized directories: it
uses discovered Git state when available and otherwise displays the project
root. This fallback cannot launch completion because dry-run never starts the
plugin.

### Explicit state meanings

`PluginConfig` carries `git_root`, with an empty default for direct fixtures and
legacy serialized values. The plugin copies it into `State.git_root` during
load.

`State.project_root` remains the Lisa root captured from Zellij's initial cwd.
It continues to own project file interpretation, hook paths, notification cwd,
and completion host-command cwd.

`State.git_root` owns the completion transaction root and Git pathspec base.
The fields are no longer overloaded.

### Path normalization

Completion mapping now converts its three accepted input representations to a
host path:

- `/host/...` becomes `<project_root>/...`;
- a relative path becomes `<project_root>/<path>`;
- a host absolute path remains absolute.

The mapper lexically normalizes path components without depending on access to
directories above the WASI mount. It then strips the normalized Git root and
rejects an empty selection or a path outside that root.

The named rejection begins with `completion path outside Git root` and contains
the offending path/root context. In production, the sole effect executor
removes pending completion state and logs the command-build error as:

`Cannot start completion for <ticket>: <named error>`

That existing activity event is dashboard-visible.

### Command contract

`build_completion_command` now emits:

- `complete-ticket --path <git-root>`;
- `--ticket-file <git-root-relative-ticket>`;
- `--work-dir <git-root-relative-work-dir>`.

For project root `/repo/games/midsummer` and Git root `/repo`, the regression
asserts exact values:

- `games/midsummer/docs/active/tickets/T-001.md`;
- `games/midsummer/docs/active/work/T-001`.

The native `complete-ticket` command retains its independent canonicalization
and repository-boundary validation as defense in depth.

## Acceptance mapping

### Discovers and retains both roots

Satisfied. The native launcher performs reliable host-side Git discovery before
WASI startup. The plugin retains that configured root separately from the Lisa
project cwd captured from Zellij.

### Uses Git root for `--path`

Satisfied by the exact argv regression
`completion_command_uses_git_root_and_nested_repository_paths`.

### Normalized Git-root-relative includes

Satisfied by `completion_repository_relative_path` and the same exact argv
regression. Sandbox, relative, and host absolute inputs converge on normalized
host paths before Git-root stripping.

### Rejects paths outside Git root visibly

Satisfied by `completion_command_rejects_path_outside_git_root` and the existing
production executor's activity-error path. The regression verifies both the
stable error name and offending path.

### Nested project prefix

Satisfied. The asserted ticket and work arguments begin with
`games/midsummer/docs/active/...`, not `docs/active/...`.

## Test coverage

Focused core configuration test:

`cargo test -p lisa-core test_config_git_root_round_trip --no-fail-fast`

Passed: 1; failed: 0.

Focused loop tests:

`cargo test -p lisa-cli loop_cmd --no-fail-fast`

Passed: 19; failed: 0. This includes a real temporary nested Git repository.

Focused completion command tests:

`cargo test -p lisa-plugin --lib completion_command --no-fail-fast`

Passed: 2; failed: 0.

Full workspace:

`cargo test --workspace --no-fail-fast`

Passed. All executed tests succeeded: 280 CLI tests, 192 core unit tests plus
core integration regressions, and 343 plugin tests. The real-Zellij boundary
test remained ignored under its declared environment contract.

WASM lint:

`cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings`

Passed.

Formatting:

`cargo fmt --all -- --check`

Passed.

## Repository review

The three ticket-owned source paths are clean after the isolated transaction.
The ordinary Git index is empty. No ordinary `git add`, broad staging command,
or ordinary `git commit` was used.

Lisa-managed ticket frontmatter, provenance, and admitted workflow artifacts
remain outside the implementation commit. The pre-existing untracked
`crates/lisa-plugin/docs/` tree was preserved.

## Open concerns and limitations

No blocking concern exists.

Real loop startup now explicitly requires Git discovery. This matches completion
transactions' existing Git requirement and avoids silently reintroducing the
nested-root ambiguity.

The layout interpolates an absolute root using the same path-display approach
already used for the WASM and Lisa binary paths. Exotic KDL quote characters in
filesystem paths are an existing broader layout-escaping concern, not introduced
as a completion-specific behavior.

Lexical normalization intentionally does not resolve symlinks inside WASI. The
native CLI canonicalizes supplied paths again before committing, so symlink
escapes remain rejected at the transaction boundary.

T-042-01-06 owns the broader nested-monorepo end-to-end regression that follows
this command-boundary fix.

## Critical issues requiring human attention

None.

## Human review focus

Confirm the root separation is maintained at the intended boundary: native code
discovers what WASI cannot reliably see, plugin state retains both meanings,
and only completion Git arguments use the enclosing repository root.

Review is complete. This attempt remains on T-042-01-05 for Lisa to admit the
Review artifacts, prepare the completion commit, publish Done, and release the
seat.
